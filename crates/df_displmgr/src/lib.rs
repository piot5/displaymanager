pub mod error;
pub mod types;
pub mod traits;
pub mod backends;

// Re-export core types for a flattened, user-friendly API.
pub use error::{DisplayError, DisplayResult};
pub use traits::{OutputEditable, UniversalTopology};
pub use types::*;

/// The primary entry point for display management.
/// Resolves to a platform-specific implementation at compile time.
///
/// # Example
/// ```rust
/// use df_displmgr::NativeTopology;
/// use df_displmgr::traits::UniversalTopology;
///
/// #[tokio::main]
/// async fn main() -> df_displmgr::DisplayResult<()> {
///     // Subsystem acquisition is synchronous and FFI-bound.
///     let mut topo = NativeTopology::acquire()?;
///     let outputs = topo.get_outputs();
///
///     // Async paths are isolated to mutations and validation passes.
///     topo.validate().await?;
///     topo.commit().await?;
///     Ok(())
/// }
/// ```
#[cfg(target_os = "windows")]
pub use crate::backends::NativeTopology;

#[cfg(target_os = "linux")]
pub use crate::backends::NativeTopology;

// Fallback for unsupported platforms — allows documentation builds and
// cross-compilation without pulling in platform-specific dependencies.
#[cfg(not(any(target_os = "windows", target_os = "linux")))]
pub struct NativeTopology;

#[cfg(not(any(target_os = "windows", target_os = "linux")))]
#[async_trait::async_trait]
impl traits::UniversalTopology for NativeTopology {
    fn acquire() -> DisplayResult<Self> {
        Err(DisplayError::UnsupportedFeature("Platform not supported".into()))
    }

    fn get_outputs(&self) -> Vec<OutputState> {
        vec![]
    }

    // FIX: parameter type was `&str`; the trait requires `&DisplayId`.
    fn edit_output(&mut self, _: &DisplayId) -> DisplayResult<Box<dyn OutputEditable + '_>> {
        Err(DisplayError::UnsupportedFeature("Platform not supported".into()))
    }

    fn set_persistence(&mut self, _: bool) -> &mut Self {
        self
    }

    async fn validate(&self) -> DisplayResult<()> {
        Ok(())
    }

    async fn commit(&mut self) -> DisplayResult<()> {
        Ok(())
    }
}