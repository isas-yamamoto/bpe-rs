use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
pub enum BpeError {
    Ok = 0,
    StreamEnd = 1,
    FileError = 2,
    StreamError = 3,
    DataError = 4,
    MemError = 5,
    BlockScanCodingError = 6,
    DynamicalRangeError = 7,
    RateError = 8,
    RateUnreachable = 9,
    WaveletInvalid = 10,
    ImageSizeWrong = 11,
    ScalingFileError = 12,
    InvalidHeader = 13,
    InvalidCodingParameters = 14,
    PatternCodingError = 15,
    RiceCodingError = 16,
    StageCodingError = 17,
}

impl BpeError {
    pub fn message(self) -> &'static str {
        match self {
            BpeError::Ok => "Success",
            BpeError::StreamEnd => "Error code 1: Bit stream end",
            BpeError::FileError => "Error code 2: File Error Msg",
            BpeError::StreamError => "Error code 3: Bitstream Error",
            BpeError::DataError => "Error code 4: Data ErrorMsg",
            BpeError::MemError => "Error code 5:  Memory allocation error",
            BpeError::BlockScanCodingError => "Error code 6: Decoding ErrorMsg",
            BpeError::DynamicalRangeError => "Error code 7: Dynamical range ErrorMsg",
            BpeError::RateError => "Error code 8: Invalid Rate",
            BpeError::RateUnreachable => "Rate  code 9: Cannot get the exact rate.",
            BpeError::WaveletInvalid => "Error code 10: Wavelet transform invalid.",
            BpeError::ImageSizeWrong => "Error code 11: Invalid image segment size.",
            BpeError::ScalingFileError => {
                "Error code 12: Scalling file open ErrorMsg or scales invalids."
            }
            BpeError::InvalidHeader => "Error code 13: Invalid header.",
            BpeError::InvalidCodingParameters => "Error code 14: Invalid Coding Parameters.",
            BpeError::PatternCodingError => "Error code 15: Pattern Coding Error.",
            BpeError::RiceCodingError => "Error code 16: Rice Coding Error.",
            BpeError::StageCodingError => "Error code 17: Stage Coding Error.",
        }
    }
}

impl fmt::Display for BpeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message())
    }
}

impl std::error::Error for BpeError {}

pub type BpeResult<T> = Result<T, BpeError>;

pub fn error_exit(err: BpeError) -> ! {
    eprintln!(" {}", err.message());
    std::process::exit(err as i32);
}
