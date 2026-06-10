self: {
  config,
  lib,
  pkgs,
  ...
}: let
  cfg = config.services.dns-server;
in {
  options.services.dns-server = {
    enable = lib.mkOption {
      type = lib.types.bool;
      default = false;
    };
    bind = lib.mkOption {
      type = lib.types.str;
      default = "127.0.0.1:53";
    };
    config = lib.mkOption {
      type = lib.types.attrs;
      default = {};
    };
  };

  config = lib.mkIf cfg.enable {
    systemd.services.dns-server = {
      wantedBy = ["nss-lookup.target" "multi-user.target"];
      before = ["nss-lookup.target"];
      after = ["network.target"];
      serviceConfig = {
        ExecStart = "${self.packages.${pkgs.stdenv.hostPlatform.system}.default}/bin/dns_server --bind 127.0.0.1:53 --config ${(pkgs.formats.toml {}).generate "dns_server_config.toml" cfg.config}";
        Restart = "on-failure";
        RestartSec = "2s";
        DynamicUser = true;
        AmbientCapabilities = ["CAP_NET_BIND_SERVICE"];
        CapabilityBoundingSet = ["CAP_NET_BIND_SERVICE"];
      };
    };
  };
}
