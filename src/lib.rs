use packed_struct::{types::bits::ByteArray, PackedStruct, PackedStructSlice, PackingError};

pub mod aarp;
pub mod addr;
pub mod ddp;
pub mod link;

use thiserror::Error;

#[derive(Error, Debug)]
pub enum CrabbletalkError {
    #[error("packed_struct error")]
    PackingError(#[from] PackingError),
    #[error("hangup")]
    Hangup,
    #[error("transient")]
    Transient,
}

pub type Result<T> = std::result::Result<T, CrabbletalkError>;

pub trait UnpackSplit {
    fn unpack_split<'d>(data: &'d [u8]) -> Result<(Self, &'d [u8])>
    where
        Self: Sized;
}

impl<T> UnpackSplit for T
where
    T: PackedStruct,
    T::ByteArray: ByteArray,
{
    fn unpack_split<'d>(data: &'d [u8]) -> Result<(Self, &'d [u8])>
    where
        Self: Sized,
    {
        let (lhs, rhs) = data.split_at(<T as PackedStruct>::ByteArray::len());
        let lhs = T::unpack_from_slice(lhs)?;
        Ok((lhs, rhs))
    }
}
