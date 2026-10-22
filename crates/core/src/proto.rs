//! Http protocol supports.
cfg_feature! {
    #![feature = "quinn"]

    pub use salvo_http3::{quic, webtransport};
    /// The actual QUIC implementation used by this Aster fork.
    pub use salvo_http3::quinn::quinn as noq;
    /// Compatibility name for [`noq`]; these are Noq types, not upstream Quinn types.
    pub use noq as quinn;
    pub use salvo_http3::webtransport::server::WebTransportSession;
}
