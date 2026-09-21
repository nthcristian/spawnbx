use super::types::{PackageIntent, PackagePlan, StatePaths};

pub(crate) trait PackagesModule {
    fn plan(
        &mut self,
        intent: &PackageIntent,
        paths: &StatePaths,
    ) -> Result<PackagePlan, PackagesError>;
}

pub(crate) struct PackagePlanner;

impl PackagePlanner {
    pub(crate) fn new() -> Self {
        Self
    }
}

impl PackagesModule for PackagePlanner {
    fn plan(
        &mut self,
        intent: &PackageIntent,
        paths: &StatePaths,
    ) -> Result<PackagePlan, PackagesError> {
        Ok(PackagePlan {
            generated_metadata: paths.nix_dir.join("flake.nix"),
            lock_path: paths.lock_path.clone(),
            profile_path: paths.home_dir.join(".spawnbx-profile"),
            container_flake_dir: "/workspace/.spawnbx/nix".to_owned(),
            container_profile_path: "/home/spawnbx/.spawnbx-profile".to_owned(),
            flake_contents: flake_contents(intent),
            lock_policy: intent.lock_policy.clone(),
            focus: None,
        })
    }
}

fn flake_contents(intent: &PackageIntent) -> String {
    let packages = intent
        .attributes
        .values
        .iter()
        .map(|package| format!("            pkgs.{}", package.attribute_path))
        .collect::<Vec<_>>()
        .join("\n");
    format!(
        "{{\n  inputs.nixpkgs.url = \"github:NixOS/nixpkgs/nixos-unstable\";\n\n  outputs = {{ nixpkgs, ... }}: let\n    pkgs = import nixpkgs {{ system = \"x86_64-linux\"; }};\n  in {{\n    packages.x86_64-linux.default = pkgs.buildEnv {{\n      name = \"spawnbx-packages\";\n      paths = [\n{}\n      ];\n    }};\n  }};\n}}\n",
        packages
    )
}

#[derive(Clone, Debug)]
pub(crate) struct PackagesError;
