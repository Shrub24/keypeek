{self, ...}: {
  flake = {
    homeManagerModules.default = {
      config,
      lib,
      pkgs,
      ...
    }: let
      cfg = config.programs.keypeek;
      settingsFormat = pkgs.formats.ini {};
      configFile = settingsFormat.generate "keypeek-settings.ini" {
        settings = builtins.mapAttrs (_: v:
          if v == true
          then "true"
          else if v == false
          then "false"
          else toString v)
        cfg.settings;
      };
    in {
      options.programs.keypeek = {
        enable = lib.mkEnableOption "KeyPeek, the keyboard layer overlay";

        package = lib.mkOption {
          type = lib.types.package;
          default = self.packages.${pkgs.stdenv.hostPlatform.system}.default;
          defaultText = lib.literalExpression ''
            keypeek.packages.''${system}.default
          '';
          description = "The KeyPeek package to use.";
        };

        settings = lib.mkOption {
          type = lib.types.attrsOf (lib.types.oneOf [
            lib.types.bool
            lib.types.int
            lib.types.float
            lib.types.str
          ]);
          default = {};
          example = lib.literalExpression ''
            {
              size = 80;
              position = "Bottom Right";
              timeout = 4000;
              legend_mode = "Single";
            }
          '';
          description = ''
            Key-value pairs written to the `[settings]` section of a
            generated read-only INI file. KeyPeek only reads this file
            when launched with `--config`; the bundled service does so
            automatically, so these settings take effect through it.
          '';
        };

        service = {
          enable = lib.mkEnableOption "the KeyPeek systemd user service";

          environment = lib.mkOption {
            type = lib.types.attrsOf lib.types.str;
            default = {};
            example = {WAYLAND_DISPLAY = "wayland-1";};
            description = ''
              Extra environment variables for the service. A systemd user
              service sees the session environment only when the compositor
              exports it (uwsm and most wlroots setups do; a bare session
              start may not), and KeyPeek needs `WAYLAND_DISPLAY` to reach
              the compositor and `DISPLAY` for its X11 fallback.
            '';
          };
        };
      };

      config = lib.mkIf cfg.enable {
        warnings =
          lib.optional (cfg.settings != {} && !cfg.service.enable)
          "programs.keypeek.settings only takes effect through the bundled service; set programs.keypeek.service.enable = true or pass --config yourself.";

        home.packages = [cfg.package];

        systemd.user.services.keypeek = lib.mkIf cfg.service.enable {
          Unit = {
            Description = "KeyPeek keyboard layer overlay";
            PartOf = ["graphical-session.target"];
            After = ["graphical-session.target"];
          };
          Service = {
            ExecStart =
              "${lib.getExe cfg.package} --no-settings"
              + lib.optionalString (cfg.settings != {}) " --config ${configFile}";
            Environment =
              lib.mapAttrsToList (name: value: "${name}=${value}")
              cfg.service.environment;
            Restart = "on-failure";
            RestartSec = 2;
          };
          Install = {WantedBy = ["graphical-session.target"];};
        };
      };
    };

    nixosModules.default = {
      config,
      lib,
      pkgs,
      ...
    }: let
      cfg = config.programs.keypeek.bleUdev;
    in {
      options.programs.keypeek.bleUdev = {
        enable = lib.mkEnableOption ''
          the packaged KeyPeek BLE udev rule, granting the active local
          user access to ZMK BLE HID devices
        '';

        package = lib.mkOption {
          type = lib.types.package;
          default = self.packages.${pkgs.stdenv.hostPlatform.system}.default;
          defaultText = lib.literalExpression ''
            keypeek.packages.''${system}.default
          '';
          description = ''
            The package whose BLE udev rule is installed. The rule lives in
            the package's `lib/udev/rules.d`, which `services.udev.packages`
            collects, so nothing is linked into `/etc` by hand.
          '';
        };
      };

      # NixOS already walks `lib/udev/rules.d` of every package listed here, and
      # `/etc/udev/rules.d` is the udev module's own to manage: a hand-written
      # entry inside it fails the `etc` derivation.
      config = lib.mkIf cfg.enable {
        services.udev.packages = [cfg.package];
      };
    };
  };
}
