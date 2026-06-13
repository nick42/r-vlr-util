//! Owned ACL inspection and scoped Win32 security descriptor construction.

use super::error::{HResult, Result};
use super::security::Sid;
use std::ffi::c_void;
use std::marker::PhantomData;
use std::mem::size_of;
use std::ptr::null;
use windows_sys::Win32::Security::{
    ACE_HEADER, ACL, ACL_SIZE_INFORMATION, AclSizeInformation, GetAce, GetAclInformation,
    INHERITED_ACE, InitializeSecurityDescriptor, IsValidAcl, PSECURITY_DESCRIPTOR,
    SECURITY_ATTRIBUTES, SECURITY_DESCRIPTOR, SetSecurityDescriptorDacl, SetSecurityDescriptorSacl,
};
use windows_sys::Win32::System::SystemServices::{
    ACCESS_ALLOWED_ACE_TYPE, ACCESS_DENIED_ACE_TYPE, SECURITY_DESCRIPTOR_REVISION,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Ace {
    pub ace_type: u8,
    pub flags: u8,
    pub bytes: Vec<u8>,
    pub access_mask: Option<u32>,
    pub sid: Option<Sid>,
}

impl Ace {
    #[must_use]
    pub const fn is_inherited(&self) -> bool {
        self.flags & INHERITED_ACE as u8 != 0
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AclSnapshot {
    pub entries: Vec<Ace>,
}

impl AclSnapshot {
    /// Copies a validated native ACL into an owned, alignment-independent view.
    ///
    /// # Safety
    ///
    /// `acl` must point to readable memory containing an ACL for this call.
    pub unsafe fn from_raw(acl: *const ACL) -> Result<Self> {
        if acl.is_null() || unsafe { IsValidAcl(acl) } == 0 {
            return Err(HResult::INVALID_ARGUMENT);
        }
        let mut info = ACL_SIZE_INFORMATION::default();
        if unsafe {
            GetAclInformation(
                acl,
                (&raw mut info).cast::<c_void>(),
                size_of::<ACL_SIZE_INFORMATION>() as u32,
                AclSizeInformation,
            )
        } == 0
        {
            return Err(HResult::last_error());
        }
        let mut entries = Vec::with_capacity(info.AceCount as usize);
        for index in 0..info.AceCount {
            let mut raw = std::ptr::null_mut();
            if unsafe { GetAce(acl, index, &raw mut raw) } == 0 {
                return Err(HResult::last_error());
            }
            let header = unsafe { &*raw.cast::<ACE_HEADER>() };
            let bytes =
                unsafe { std::slice::from_raw_parts(raw.cast::<u8>(), header.AceSize as usize) }
                    .to_vec();
            let access_mask = matches!(
                u32::from(header.AceType),
                ACCESS_ALLOWED_ACE_TYPE | ACCESS_DENIED_ACE_TYPE
            )
            .then(|| u32::from_le_bytes(bytes[4..8].try_into().expect("ACE contains mask")));
            let sid = matches!(
                u32::from(header.AceType),
                ACCESS_ALLOWED_ACE_TYPE | ACCESS_DENIED_ACE_TYPE
            )
            .then(|| unsafe { Sid::from_raw_copy(raw.cast::<u8>().add(8).cast()) })
            .transpose()?;
            entries.push(Ace {
                ace_type: header.AceType,
                flags: header.AceFlags,
                bytes,
                access_mask,
                sid,
            });
        }
        Ok(Self { entries })
    }

    #[must_use]
    pub fn entirely_inherited(&self) -> bool {
        self.entries.iter().all(Ace::is_inherited)
    }

    #[must_use]
    pub fn effectively_identical(&self, other: &Self) -> bool {
        self.entries == other.entries
    }
}

pub struct SecurityDescriptor<'a> {
    raw: SECURITY_DESCRIPTOR,
    _borrowed_acl: PhantomData<&'a ACL>,
}

impl SecurityDescriptor<'_> {
    pub fn new() -> Result<Self> {
        let mut raw = SECURITY_DESCRIPTOR::default();
        if unsafe {
            InitializeSecurityDescriptor(
                (&raw mut raw).cast::<c_void>(),
                SECURITY_DESCRIPTOR_REVISION,
            )
        } == 0
        {
            return Err(HResult::last_error());
        }
        Ok(Self {
            raw,
            _borrowed_acl: PhantomData,
        })
    }

    pub fn clear_dacl(&mut self) -> Result<()> {
        self.set_dacl_pointer(null(), false, false)
    }

    pub fn clear_sacl(&mut self) -> Result<()> {
        self.set_sacl_pointer(null(), false, false)
    }

    /// Sets a borrowed native DACL.
    ///
    /// # Safety
    ///
    /// `acl` must remain valid for the descriptor's lifetime. A null ACL must
    /// only be supplied when `allow_null` is true.
    pub unsafe fn set_dacl(
        &mut self,
        acl: *const ACL,
        defaulted: bool,
        allow_null: bool,
    ) -> Result<()> {
        if acl.is_null() && !allow_null {
            return Err(HResult::INVALID_ARGUMENT);
        }
        self.set_dacl_pointer(acl, true, defaulted)
    }

    /// Sets a borrowed native SACL.
    ///
    /// # Safety
    ///
    /// `acl` must remain valid for the descriptor's lifetime. A null ACL must
    /// only be supplied when `allow_null` is true.
    pub unsafe fn set_sacl(
        &mut self,
        acl: *const ACL,
        defaulted: bool,
        allow_null: bool,
    ) -> Result<()> {
        if acl.is_null() && !allow_null {
            return Err(HResult::INVALID_ARGUMENT);
        }
        self.set_sacl_pointer(acl, true, defaulted)
    }

    fn set_dacl_pointer(&mut self, acl: *const ACL, present: bool, defaulted: bool) -> Result<()> {
        if unsafe {
            SetSecurityDescriptorDacl(self.as_raw(), i32::from(present), acl, i32::from(defaulted))
        } == 0
        {
            Err(HResult::last_error())
        } else {
            Ok(())
        }
    }

    fn set_sacl_pointer(&mut self, acl: *const ACL, present: bool, defaulted: bool) -> Result<()> {
        if unsafe {
            SetSecurityDescriptorSacl(self.as_raw(), i32::from(present), acl, i32::from(defaulted))
        } == 0
        {
            Err(HResult::last_error())
        } else {
            Ok(())
        }
    }

    #[must_use]
    pub fn as_raw(&mut self) -> PSECURITY_DESCRIPTOR {
        (&raw mut self.raw).cast()
    }
}

pub struct SecurityAttributes<'a> {
    raw: SECURITY_ATTRIBUTES,
    _descriptor: PhantomData<&'a mut SECURITY_DESCRIPTOR>,
}

impl<'a> SecurityAttributes<'a> {
    #[must_use]
    pub fn new(inherit_handle: bool) -> Self {
        Self {
            raw: SECURITY_ATTRIBUTES {
                nLength: size_of::<SECURITY_ATTRIBUTES>() as u32,
                lpSecurityDescriptor: std::ptr::null_mut(),
                bInheritHandle: i32::from(inherit_handle),
            },
            _descriptor: PhantomData,
        }
    }

    #[must_use]
    pub fn with_descriptor(mut self, descriptor: &'a mut SecurityDescriptor<'a>) -> Self {
        self.raw.lpSecurityDescriptor = descriptor.as_raw();
        self
    }

    #[must_use]
    pub fn as_raw(&mut self) -> *mut SECURITY_ATTRIBUTES {
        &raw mut self.raw
    }
}

#[cfg(test)]
mod tests {
    use super::{AclSnapshot, SecurityAttributes, SecurityDescriptor};
    use std::mem::size_of;
    use windows_sys::Win32::Security::{ACL, ACL_REVISION, InitializeAcl};

    #[test]
    fn initializes_and_clears_security_descriptor_acls() {
        let mut descriptor = SecurityDescriptor::new().unwrap();
        descriptor.clear_dacl().unwrap();
        descriptor.clear_sacl().unwrap();
        let mut attributes = SecurityAttributes::new(true).with_descriptor(&mut descriptor);
        assert!(!attributes.as_raw().is_null());
    }

    #[test]
    fn snapshots_empty_native_acl() {
        let word_count = size_of::<ACL>().div_ceil(size_of::<usize>());
        let mut storage = vec![0_usize; word_count];
        let acl = storage.as_mut_ptr().cast::<ACL>();
        // SAFETY: storage is aligned and writable for the supplied size.
        assert_ne!(
            unsafe { InitializeAcl(acl, size_of::<ACL>() as u32, ACL_REVISION) },
            0
        );
        // SAFETY: InitializeAcl created a valid ACL in storage.
        let snapshot = unsafe { AclSnapshot::from_raw(acl) }.unwrap();
        assert!(snapshot.entries.is_empty());
        assert!(snapshot.entirely_inherited());
    }
}
