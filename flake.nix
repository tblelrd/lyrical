{
  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
  };

  outputs = {nixpkgs, ...}: 
  let
    system = "x86_64-linux";
    pkgs = nixpkgs.legacyPackages.${system};
  in
  {
    devShells.${system}.default =
        pkgs.mkShell {
          packages = [
            # https://www.reddit.com/r/NixOS/comments/1bdh1qs/openssl_with_rust_help/
            pkgs.pkg-config
            pkgs.openssl
          ];
      };
  };
}
