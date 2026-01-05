{ lib }:
lib.makeExtensible (
  self:
  let
    inherit (lib)
      attrNames
      concatLines
      concatStrings
      concatStringsSep
      drop
      filter
      filterAttrs
      hasSuffix
      id
      isList
      last
      mapAttrs'
      removePrefix
      removeSuffix
      sort
      splitString
      stringToCharacters
      take
      versionOlder
      nameValuePair
      ;
    inherit (builtins)
      match
      pathExists
      readDir
      replaceStrings
      typeOf
      ;
  in
  rec {
    chain = {
      func = id;
      __functor =
        self: input:
        if (typeOf input) == "lambda" then self // { func = e: input (self.func e); } else self.func input;
    };

    isNormalVersion = v: isList (match "([[:digit:]]+\.[[:digit:]]+(\.[[:digit:]]+)?)" v);

    latestVersion =
      versions: chain (filter isNormalVersion) (sort versionOlder) last (attrNames versions);

    escapeVersion = replaceStrings [ "." " " ] [ "_" "_" ];

    removeVanilla = n: escapeVersion (removePrefix "vanilla-" n);

    # Stolen from digga: https://github.com/divnix/digga/blob/587013b2500031b71959496764b6fdd1b2096f9a/src/importers.nix#L61-L114
    rakeLeaves =
      dirPath:
      let
        seive =
          file: type:
          # Only rake `.nix` files or directories
          (type == "regular" && hasSuffix ".nix" file) || (type == "directory");

        collect = file: type: {
          name = removeSuffix ".nix" file;
          value =
            let
              path = dirPath + "/${file}";
            in
            if (type == "regular") || (type == "directory" && pathExists (path + "/default.nix")) then
              path
            # recurse on directories that don't contain a `default.nix`
            else
              rakeLeaves path;
        };

        files = filterAttrs seive (readDir dirPath);
      in
      filterAttrs (n: v: v != { }) (mapAttrs' collect files);

    # Same as collectFiles, but only gathers files from a specific subdirectory
    # (e.g. "config")
    collectFilesAt =
      path: subdir: mapAttrs' (n: nameValuePair ("${subdir}/${n}")) (collectFiles "${path}/${subdir}");

    # Get all files from a path (e.g. a modpack derivation) and return them in the
    # format expected by the files/symlinks module options.
    collectFiles =
      let
        mapListToAttrs =
          fn: fv: list:
          lib.listToAttrs (map (x: nameValuePair (fn x) (fv x)) list);
      in
      path:
      mapListToAttrs (x: builtins.unsafeDiscardStringContext (lib.removePrefix "${path}/" x)) (lib.id) (
        lib.filesystem.listFilesRecursive "${path}"
      );

    wrapJarManifest =
      manifestText:
      let
        chunkCharacters' =
          characters: chunks:
          if
            characters != [ ] # 71 characters plus space prefix = 72 line length
          then
            chunkCharacters' (drop 71 characters) (chunks ++ [ (take 71 characters) ])
          else
            chunks;

        chunkCharacters =
          characters: # First line gets 72 due to not having space prefix
          chunkCharacters' (drop 72 characters) [ (take 72 characters) ];

        wrapLine = chain stringToCharacters chunkCharacters (map concatStrings) (concatStringsSep "\n ");
      in
      chain (splitString "\n") (map wrapLine) concatLines manifestText;
    
    nonEmpty = x: x != { } && x != [ ];
    nonEmptyValue = x: nonEmpty x && (x ? value -> nonEmpty x.value);

    txtList = pkgs: { }: {
      type = with lib.types; listOf str;
      generate = name: value: pkgs.writeText name (lib.concatStringsSep "\n" value);
    };

    formatExtensions = pkgs: with pkgs.formats; {
      "yml" = yaml { };
      "yaml" = yaml { };
      "json" = json { };
      "props" = keyValue { };
      "properties" = keyValue { };
      "toml" = toml { };
      "ini" = ini { };
      "txt" = txtList pkgs { };
    };

    inferFormat = pkgs: name:
      let
        error = throw "nix-minecraft: Could not infer format from file '${name}'. Specify one using 'format'.";
        extension = builtins.match "[^.]*\\.(.+)" name;
      in
      if extension != null && extension != [ ] then
        (formatExtensions pkgs).${lib.head extension} or error
      else
        error;

    getFormat = pkgs: name: config:
      if config ? format && config.format != null then config.format else inferFormat pkgs name;

    configToPath = pkgs: name: config:
      if lib.isStringLike config then
        config
      else
        (getFormat pkgs name config).generate name config.value;

    normalizeFiles = pkgs: files: lib.mapAttrs (configToPath pkgs) (lib.filterAttrs (_: nonEmptyValue) files);

    # Produces a directory containing all the files and symlinks for a server
    mkServerData =
      {
        pkgs,
        serverProperties ? { },
        symlinks ? { },
        files ? { },
        whitelist ? { },
        operators ? { },
        ...
      }:
      let
        allSymlinks = normalizeFiles pkgs (
          {
            "eula.txt".value = {
              eula = true;
            };
            "eula.txt".format = pkgs.formats.keyValue { };
          }
          // symlinks
        );
        allFiles = normalizeFiles pkgs (
          {
            "whitelist.json".value = lib.mapAttrsToList (n: v: {
              name = n;
              uuid = v;
            }) whitelist;
            "ops.json".value = lib.mapAttrsToList (n: v: {
              name = n;
              uuid = v.uuid;
              level = v.level;
              bypassesPlayerLimit = v.bypassesPlayerLimit;
            }) operators;
            "server.properties".value = serverProperties;
          }
          // files
        );
      in
      pkgs.runCommand "server-data" { } ''
        mkdir -p $out
        ${lib.concatStringsSep "\n" (
          lib.mapAttrsToList (n: v: ''
            mkdir -p "$out/$(dirname "${n}")"
            ln -s "${v}" "$out/${n}"
          '') (allSymlinks // allFiles)
        )}
      '';

    # Builds an OCI image for a Minecraft server
    buildImage =
      {
        pkgs,
        package,
        mc-manager,
        flavor ? "vanilla",
        jvmOpts ? "-Xmx2G -Xms1G",
        serverProperties ? { },
        symlinks ? { },
        files ? { },
        whitelist ? { },
        operators ? { },
        imageName ? "minecraft-server",
        tag ? "latest",
        debug ? false,
        extraContents ? [ ],
      }:
      let
        useMods = lib.elem flavor [ "fabric" "forge" ] || lib.hasPrefix "modpack" flavor;
        mc-manager-final = mc-manager.override {
          buildFeatures = if useMods then [ "mods" ] else [ ];
        };

        # Extract attributes, favoring passthru if present
        actualVanillaJar = package.passthru.vanillaJar or (package.vanillaJar or "${package}/lib/minecraft/server.jar");
        actualLoaderJar = package.passthru.loaderJar or (package.loaderJar or null);
        actualLoader = package.passthru.loader or (package.loader or null);
        actualVanillaServer = package.passthru.vanilla-server or (package.vanilla-server or null);

        # Create a stable directory structure for the container to reference
        mc-artifacts = pkgs.runCommand "mc-artifacts" { } ''
          mkdir -p $out/mc-artifacts
          ln -s ${actualVanillaJar} $out/mc-artifacts/server.jar
          ${lib.optionalString (actualLoaderJar != null) ''
            ln -s ${actualLoaderJar} $out/mc-artifacts/loader.jar
          ''}
          ln -s ${serverData} $out/mc-artifacts/server-data
        '';

        serverData = mkServerData {
          inherit
            pkgs
            serverProperties
            symlinks
            files
            whitelist
            operators
            ;
        };
        # Helper to inflate the closure of serverData (to catch symlink targets)
        inflatedData = pkgs.runCommand "inflated-data" { } ''
          mkdir -p $out
          cp -rL ${serverData}/* $out/
        '';
      in
      pkgs.dockerTools.buildLayeredImage {
        name = imageName;
        inherit tag;
        contents =
          [
            pkgs.jre_headless
            mc-manager-final
            mc-artifacts
            package
            inflatedData
          ]
          ++ (lib.optional (actualVanillaServer != null) actualVanillaServer)
          ++ (lib.optional (actualLoader != null) actualLoader)
          ++ (lib.optionals debug [
            pkgs.coreutils
            pkgs.bash
            pkgs.busybox
          ])
          ++ extraContents;
        config = {
          Cmd = [ (lib.getExe mc-manager-final) ];
          Env = [
            "FLAVOR=${flavor}"
            "VANILLA_JAR=/mc-artifacts/server.jar"
            "LOADER_JAR=${if (actualLoaderJar != null) then "/mc-artifacts/loader.jar" else ""}"
            "JAVA_BIN=${pkgs.jre_headless}/bin/java"
            "MODS_DIR=/mc-artifacts/server-data/mods"
            "OVERRIDES_DIR=/mc-artifacts/server-data"
            "EULA=FALSE"
          ];
          WorkingDir = "/data";
          Volumes = {
            "/data" = { };
          };
        };
      };
  }
)
