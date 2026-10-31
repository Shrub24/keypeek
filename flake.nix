{
  description = "keypeek";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";

    crane.url = "github:ipetkov/crane";

    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
      inputs.rust-analyzer-src.follows = "";
    };

    flake-utils.url = "github:numtide/flake-utils";

    flake-compat = {
      url = "github:edolstra/flake-compat";
      flake = false;
    };

    advisory-db = {
      url = "github:rustsec/advisory-db";
      flake = false;
    };

    pre-commit = {
      url = "github:cachix/pre-commit-hooks.nix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = {
    self,
    nixpkgs,
    crane,
    fenix,
    flake-utils,
    #advisory-db,
    pre-commit,
    ...
  }:
    flake-utils.lib.eachDefaultSystem (system: let
      pkgs = nixpkgs.legacyPackages.${system};

      inherit (pkgs) lib;

      libPath = with pkgs;
        lib.makeLibraryPath [
          libGL
          libxkbcommon
          wayland
          libx11
          libxcursor
          libxi
          libxrandr
          libayatana-appindicator
        ];

      craneLib = crane.mkLib pkgs;
      src = lib.cleanSourceWith {
        src = ./.;
        # filter out all non rust and still include resources
        filter = path: type:
          (builtins.match ".+-source/resources/.*$" path != null) || (craneLib.filterCargoSources path type);
        name = "source";
      };

      # Common arguments can be set here to avoid repeating them later
      commonArgs = {
        inherit src;
        strictDeps = false; # else openssl-sys fails

        buildInputs = with pkgs; [
          glib
          dbus
          atk
          gdk-pixbuf
          pango
          gtk3
          udev
          xdotool
          pkg-config

          libxcb

          wayland
          libGL
          libxkbcommon
        ];

        # Additional environment variables can be set directly
        # MY_CUSTOM_VAR = "some value";
      };

      craneLibLLvmTools =
        craneLib.overrideToolchain
        (fenix.packages.${system}.stable.withComponents [
          "cargo"
          "llvm-tools"
          "rustc"
        ]);

      # Build *just* the cargo dependencies, so we can reuse
      # all of that work (e.g. via cachix) when running in CI
      cargoArtifacts = craneLib.buildDepsOnly commonArgs;

      # Build the actual crate itself, reusing the dependency
      # artifacts from above.
      keypeek = craneLib.buildPackage (commonArgs
        // {
          inherit cargoArtifacts;
          doCheck = false;
          meta.mainProgram = "keypeek";
        }
        // {
          nativeBuildInputs = [pkgs.makeWrapper];
          postInstall =
            if pkgs.stdenv.isLinux
            then ''
              wrapProgram "$out/bin/keypeek" --prefix LD_LIBRARY_PATH : "${libPath}"

              # Desktop and BLE integration assets; see README (Linux permissions).
              install -Dm644 ${./cargo-appimage.desktop} \
                $out/share/applications/keypeek.desktop
              substituteInPlace $out/share/applications/keypeek.desktop \
                --replace-fail "Exec=keypeek" "Exec=$out/bin/keypeek"
              install -Dm644 ${./resources/icon.svg} \
                $out/share/icons/hicolor/scalable/apps/keypeek.svg
              install -Dm644 ${./resources/udev/60-keypeek.rules} \
                $out/lib/udev/rules.d/60-keypeek.rules
            ''
            else ''
              wrapProgram "$out/bin/keypeek" --prefix LD_LIBRARY_PATH : "${libPath}"
            '';
        });
    in {
      checks = {
        # Build the crate as part of `nix flake check` for convenience
        inherit keypeek;
        # Run clippy (and deny all warnings) on the crate source,
        # again, reusing the dependency artifacts from above.
        #
        # Note that this is done as a separate derivation so that
        # we can block the CI if there are issues here, but not
        # prevent downstream consumers from building our crate by itself.
        #keypeek-clippy = craneLib.cargoClippy (commonArgs
        #  // {
        #    inherit cargoArtifacts;
        #    cargoClippyExtraArgs = "--all-targets -- --deny warnings";
        #  });

        #keypeek-doc = craneLib.cargoDoc (commonArgs
        #  // {
        #    inherit cargoArtifacts;
        #  });

        # Check formatting
        #keypeek-fmt = craneLib.cargoFmt {
        #  inherit src;
        #};

        # Audit dependencies
        #keypeek-audit = craneLib.cargoAudit {
        #  inherit src advisory-db;
        #};

        # Audit licenses
        #keypeek-deny = craneLib.cargoDeny {
        #  inherit src;
        #};

        # Run tests with cargo-nextest
        keypeek-nextest = craneLib.cargoNextest (commonArgs
          // {
            inherit cargoArtifacts;
            partitions = 1;
            partitionType = "count";
            cargoNextestExtraArgs = "--workspace --no-tests=pass";
          });

        pre-commit-check = pre-commit.lib.${system}.run {
          src = ./.;
          hooks = {
            #editorconfig-checker.enable = true;
            alejandra.enable = true;
            deadnix.enable = true;
            #flake-checker.enable = true;
            statix.enable = true;
          };
        };

        # Minimal test configurations that keep the modules evaluating.
        # The HM/system-manager option sets are stubbed with the subset the
        # modules touch, so the flake stays free of those inputs.
        keypeek-hm-module = let
          evaled = lib.evalModules {
            specialArgs = {inherit pkgs;};
            modules = [
              self.homeManagerModules.default
              {
                options.home.packages = lib.mkOption {
                  type = lib.types.listOf lib.types.package;
                  default = [];
                };
                options.systemd.user.services = lib.mkOption {
                  type = lib.types.attrsOf lib.types.anything;
                  default = {};
                };
              }
              {
                programs.keypeek = {
                  enable = true;
                  settings.size = 80;
                  service.enable = true;
                };
              }
            ];
          };
        in
          pkgs.runCommand "keypeek-hm-module" {}
          ''
            test -n "${toString (lib.length evaled.config.home.packages)}"
            test -e "$(echo ${evaled.config.systemd.user.services.keypeek.Service.ExecStart} | cut -d' ' -f1)"
            touch $out
          '';

        keypeek-sm-module = let
          evaled = lib.evalModules {
            specialArgs = {inherit pkgs;};
            modules = [
              self.systemManagerModules.default
              {
                options.environment.etc = lib.mkOption {
                  type = lib.types.attrsOf (lib.types.submodule {
                    options.source = lib.mkOption {type = lib.types.nullOr lib.types.path;};
                  });
                  default = {};
                };
              }
              {programs.keypeek.bleUdev.enable = true;}
            ];
          };
        in
          pkgs.runCommand "keypeek-sm-module" {}
          ''
            test -e "${evaled.config.environment.etc."udev/rules.d/60-keypeek.rules".source}"
            touch $out
          '';
      };

      packages =
        {
          default = keypeek;
        }
        // lib.optionalAttrs (!pkgs.stdenv.isDarwin) {
          keypeek-llvm-coverage = craneLibLLvmTools.cargoLlvmCov (commonArgs
            // {
              inherit cargoArtifacts;
            });
        };

      apps.default = flake-utils.lib.mkApp {
        drv = keypeek;
      };

      devShells.default = craneLib.devShell {
        # Inherit inputs from checks.
        checks = self.checks.${system};

        # Additional dev-shell environment variables can be set directly
        # MY_CUSTOM_DEVELOPMENT_VAR = "something else";
        LD_LIBRARY_PATH = libPath;

        # Extra inputs can be added here; cargo and rustc are provided by default.
        packages = [];
      };
    })
    // {
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

          service.enable = lib.mkEnableOption "the KeyPeek systemd user service";
        };

        config = lib.mkIf cfg.enable {
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
              Restart = "on-failure";
              RestartSec = 2;
            };
            Install = {WantedBy = ["graphical-session.target"];};
          };
        };
      };

      systemManagerModules.default = {
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
              The package whose BLE udev rule is linked into
              `/etc/udev/rules.d`.
            '';
          };
        };

        config = lib.mkIf cfg.enable {
          environment.etc."udev/rules.d/60-keypeek.rules".source = "${cfg.package}/lib/udev/rules.d/60-keypeek.rules";
        };
      };
    };
}
