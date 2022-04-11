/// Astar XCM tools.
#[derive(Debug, clap::Parser)]
#[clap(subcommand_required = true)]
pub struct Cli {
    /// Possible subcommand with parameters.
    #[clap(subcommand)]
    pub subcommand: Option<Subcommand>,
}

/// Possible subcommands of the main binary.
#[derive(Debug, clap::Subcommand)]
pub enum Subcommand {
    /// Prints parachain AccountId.
    ParachainAccount(ParachainAccountCmd),
    /// Prints AssetId for desired parachain asset.
    AssetId(AssetIdCmd),
}

/// Helper that prints AccountId of parachain.
#[derive(Debug, clap::Parser)]
pub struct ParachainAccountCmd {
    /// Print address for sibling parachain [child by default].
    #[clap(short)]
    pub sibling: bool,

    /// Target ParaId.
    pub parachain_id: u32,
}

/// Helper that prints AssetId for sibling parachain asset.
#[derive(Debug, clap::Parser)]
pub struct AssetIdCmd {
    /// External ParaId.
    pub parachain_id: u32,

    /// External AssetId.
    pub asset_id: u32,
}
