//! Video processing pipelines.

mod encoder_state;
mod shared;

pub use shared::{
    EncodedVideoFrame, PipelineAppliedConfig, PipelineLifecycle, PipelineStateNotification,
    SharedVideoPipeline, SharedVideoPipelineConfig, SharedVideoPipelineStats,
};
