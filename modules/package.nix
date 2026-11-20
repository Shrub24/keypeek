{
  inputs,
  self,
  ...
}: {
  perSystem = {
    config,
    pkgs,
    lib,
    system,
    ...
  }: let
    inherit (inputs) crane pre-commit;
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
      src = ../.;
      # filter out all non rust and still include resources
      filter = path: type:
        (builtins.match ".+-source/resources/.*$" path != null) || (craneLib.filterCargoSources path type);
      name = "source";
    };

    # The workspace root is a virtual manifest, so crane cannot read a name or
    # version from it; both are stated here instead.
    cargoToml = builtins.fromTOML (builtins.readFile ../Cargo.toml);

    # Common arguments can be set here to avoid repeating them later
    commonArgs = {
      inherit src;
      pname = "keypeek";
      inherit (cargoToml.workspace.package) version;
      strictDeps = true;
      nativeBuildInputs = [pkgs.pkg-config];

      buildInputs = with pkgs; [
        glib
        dbus
        atk
        gdk-pixbuf
        pango
        gtk3
        udev
        xdotool

        libxcb

        wayland
        libGL
        libxkbcommon
      ];
    };

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
        nativeBuildInputs = commonArgs.nativeBuildInputs ++ [pkgs.makeWrapper];
        postInstall =
          if pkgs.stdenv.hostPlatform.isLinux
          then ''
            wrapProgram "$out/bin/keypeek" --prefix LD_LIBRARY_PATH : "${libPath}"

            # Desktop and BLE integration assets; see README (Linux permissions).
            install -Dm644 ${../cargo-appimage.desktop} \
              $out/share/applications/keypeek.desktop
            substituteInPlace $out/share/applications/keypeek.desktop \
              --replace-fail "Exec=keypeek" "Exec=$out/bin/keypeek --settings"
            install -Dm644 ${../resources/icon.svg} \
              $out/share/icons/hicolor/scalable/apps/keypeek.svg
            install -Dm644 ${../resources/udev/60-keypeek.rules} \
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
      # Run tests with cargo-nextest
      keypeek-nextest = craneLib.cargoNextest (commonArgs
        // {
          inherit cargoArtifacts;
          partitions = 1;
          partitionType = "count";
          cargoNextestExtraArgs = "--workspace --exclude keypeek-web";
        });

      pre-commit-check = pre-commit.lib.${system}.run {
        src = ../.;
        hooks = {
          alejandra.enable = true;
          deadnix.enable = true;
          statix.enable = true;
        };
      };

      # Minimal test configurations that keep the modules evaluating.
      # The HM/NixOS option sets are stubbed with the subset the
      # modules touch, so the flake stays free of those inputs.
      keypeek-hm-module = let
        evaled = lib.evalModules {
          specialArgs = {inherit pkgs;};
          modules = [
            self.homeManagerModules.default
            {
              options = {
                home.packages = lib.mkOption {
                  type = lib.types.listOf lib.types.package;
                  default = [];
                };
                systemd.user.services = lib.mkOption {
                  type = lib.types.attrsOf lib.types.anything;
                  default = {};
                };
                warnings = lib.mkOption {
                  type = lib.types.listOf lib.types.str;
                  default = [];
                };
              };
            }
            {
              programs.keypeek = {
                enable = true;
                settings.size = 80;
                service = {
                  enable = true;
                  environment.WAYLAND_DISPLAY = "wayland-1";
                };
              };
            }
          ];
        };
      in
        pkgs.runCommand "keypeek-hm-module" {}
        ''
          test -n "${toString (lib.length evaled.config.home.packages)}"
          test -e "$(echo ${evaled.config.systemd.user.services.keypeek.Service.ExecStart} | cut -d' ' -f1)"
          test -n "${toString (lib.length evaled.config.systemd.user.services.keypeek.Service.Environment)}"
          touch $out
        '';

      keypeek-nixos-module = let
        evaled = lib.evalModules {
          specialArgs = {inherit pkgs;};
          modules = [
            self.nixosModules.default
            {
              options.services.udev.packages = lib.mkOption {
                type = lib.types.listOf lib.types.package;
                default = [];
              };
            }
            {programs.keypeek.bleUdev.enable = true;}
          ];
        };
      in
        pkgs.runCommand "keypeek-nixos-module" {}
        ''
          test -e "${builtins.head evaled.config.services.udev.packages}/lib/udev/rules.d/60-keypeek.rules"
          touch $out
        '';
    };

    packages.default = keypeek;
    apps.default = {
      type = "app";
      program = lib.getExe keypeek;
      meta.description = "Live keyboard layer overlay";
    };

    devShells.default = craneLib.devShell {
      # Inherit inputs from checks.
      inherit (config) checks;

      # MY_CUSTOM_DEVELOPMENT_VAR = "something else";
      LD_LIBRARY_PATH = libPath;
    };
  };
}
