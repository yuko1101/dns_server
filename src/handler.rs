use std::{
    borrow::Cow,
    net::{IpAddr, Ipv4Addr, Ipv6Addr},
};

use hickory_server::{
    authority::MessageResponseBuilder,
    proto::{
        op::{Header, MessageType, OpCode, ResponseCode},
        rr::{IntoName, Name, RData, Record, RecordType, rdata},
    },
    server::{Request, RequestHandler, ResponseHandler, ResponseInfo},
};

use crate::config::Config;

pub struct CustomRequestHandler {
    config: Config,
}

impl CustomRequestHandler {
    pub fn new(config: Config) -> Self {
        Self { config }
    }

    pub async fn handle_query_request(
        &self,
        request: &Request,
        mut response_handle: impl ResponseHandler,
    ) -> std::io::Result<ResponseInfo> {
        let Some(query) = request.queries().first() else {
            return self
                .send_error_response(&mut response_handle, request, ResponseCode::FormErr)
                .await;
        };

        let Ok(domain_name) = query.name().into_name() else {
            return self
                .send_error_response(&mut response_handle, request, ResponseCode::FormErr)
                .await;
        };
        let answers = match self.resolve_query(domain_name, query.query_type()).await {
            QueryResult::Records(records) => records,
            QueryResult::NXDomain => {
                return self
                    .send_error_response(&mut response_handle, request, ResponseCode::NXDomain)
                    .await;
            }
        };

        let response = MessageResponseBuilder::from_message_request(request).build(
            self.get_response_header(request.header()),
            answers.iter(),
            &[],
            &[],
            &[],
        );

        response_handle.send_response(response).await
    }

    async fn resolve_query(&self, name: Name, record_type: RecordType) -> QueryResult {
        let Some(domain) = self.config.domains.get(&name) else {
            return self.handle_fallback_query(name, record_type).await;
        };

        let records = domain
            .records
            .iter()
            .filter(|r| r.rdata.record_type() == record_type);
        let records = records
            .map(|record| {
                Record::from_rdata(
                    name.clone(),
                    300,
                    rdata_translate_interface(&record.rdata, &record.interface_translation),
                )
            })
            .collect();
        QueryResult::Records(records)
    }

    async fn handle_fallback_query(&self, name: Name, record_type: RecordType) -> QueryResult {
        let fallbacks = &self.config.fallbacks;
        for fallback in fallbacks {
            let result = match fallback.lookup(name.clone(), record_type).await {
                Ok(lookup) => lookup,
                Err(e) => {
                    if e.is_no_records_found() {
                        return QueryResult::Records(vec![]);
                    } else if e.is_nx_domain() {
                        return QueryResult::NXDomain;
                    } else {
                        continue;
                    }
                }
            };

            let records = result
                .record_iter()
                .map(|record| {
                    let name = record.name().clone();
                    let ttl = record.ttl();
                    let rdata = record.data().clone();
                    Record::from_rdata(name, ttl, rdata)
                })
                .collect();

            return QueryResult::Records(records);
        }

        QueryResult::NXDomain
    }

    fn get_response_header(&self, request_header: &Header) -> Header {
        let mut response_header = Header::response_from_request(request_header);
        response_header.set_recursion_available(true);
        response_header
    }

    async fn send_error_response(
        &self,
        response_handle: &mut impl ResponseHandler,
        request: &Request,
        response_code: ResponseCode,
    ) -> std::io::Result<ResponseInfo> {
        let response = MessageResponseBuilder::from_message_request(request);
        response_handle
            .send_response(
                response.error_msg(&self.get_response_header(request.header()), response_code),
            )
            .await
    }
}

#[async_trait::async_trait]
impl RequestHandler for CustomRequestHandler {
    async fn handle_request<R: ResponseHandler>(
        &self,
        request: &Request,
        mut response_handle: R,
    ) -> ResponseInfo {
        let result = match request.message_type() {
            MessageType::Query => match request.op_code() {
                OpCode::Query => self.handle_query_request(request, response_handle).await,
                _ => {
                    self.send_error_response(&mut response_handle, request, ResponseCode::NotImp)
                        .await
                }
            },
            MessageType::Response => {
                self.send_error_response(&mut response_handle, request, ResponseCode::NotImp)
                    .await
            }
        };

        result.unwrap_or_else(|_| {
            let mut header = self.get_response_header(request.header());
            header.set_response_code(ResponseCode::ServFail);
            header.into()
        })
    }
}

pub enum QueryResult {
    Records(Vec<Record>),
    NXDomain,
}

fn rdata_translate_interface(rdata: &RData, interface: &Option<String>) -> RData {
    let Some(interface) = interface else {
        return rdata.clone();
    };
    match rdata {
        RData::A(ipv4) => RData::A(rdata::A(
            ipv4_translate_interface(ipv4, interface).into_owned(),
        )),
        RData::AAAA(ipv6) => RData::AAAA(rdata::AAAA(
            ipv6_translate_interface(ipv6, interface).into_owned(),
        )),
        _ => rdata.clone(),
    }
}

fn ipv4_translate_interface<'a>(ipv4: &'a Ipv4Addr, interface: &str) -> Cow<'a, Ipv4Addr> {
    if !ipv4.is_loopback() {
        return Cow::Borrowed(ipv4);
    }
    let ls = local_ip_address::list_afinet_netifas().unwrap_or_default();
    for (name, ip) in ls {
        if name == interface {
            if let IpAddr::V4(translated) = ip {
                return Cow::Owned(translated);
            }
        }
    }
    Cow::Borrowed(ipv4)
}

fn ipv6_translate_interface<'a>(ipv6: &'a Ipv6Addr, interface: &str) -> Cow<'a, Ipv6Addr> {
    if !ipv6.is_loopback() {
        return Cow::Borrowed(ipv6);
    }
    let ls = local_ip_address::list_afinet_netifas().unwrap_or_default();
    for (name, ip) in ls {
        if name == interface {
            if let IpAddr::V6(translated) = ip {
                return Cow::Owned(translated);
            }
        }
    }
    Cow::Borrowed(ipv6)
}
