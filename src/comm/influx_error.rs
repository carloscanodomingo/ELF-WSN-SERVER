use std::error::*;
use std::fmt;
#[derive(Debug, PartialEq, Copy, Clone)]
pub enum InfluxErrorCode {
    HeaderLengthIncorrect,
    DecodeHeaderError,
    NumBuffersFrac,
    IncorrectNumBuffers,
    DecodeHeaderDataError,
    DataIdDuplicated,
    DecodeHeaderCmdError,
    CmdIncorrectLength,
    CmdTypeDoesntMatch,
    CmdTypeNotImplemented,
    RespCmdNotImplemented,
    EncodeCmdError,
    RxLockFailed,
    IdDuplicated,
    IdNotValid,
    IdDbContext,
    TimeError,
    NotPreviousSync,
    WriteLockFailed,
    ConfigFileNotFound,
    UniqueIdNotRegistered,
    TimeBeforeServerTime,
    SamplePeriodNotDiv,
    TimeBeforeLastSync,
    NotValidConfigFile,
    ConfigPathDoesntExits,
}

impl fmt::Display for InfluxErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}
impl Error for InfluxErrorCode {}
