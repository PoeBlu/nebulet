use super::{UserHandle, Handle, HandleRights};
use nil::{Ref, HandleRef};
use nil::mem::Array;

use nabi::{Result, Error};

pub struct HandleTable {
    /// Raw array of handles,
    array: Array<Option<Handle<HandleRef>>>,
    /// Stack/queue of free indices.
    free_indices: Array<usize>,
}

impl HandleTable {
    pub fn new() -> HandleTable {
        HandleTable {
            array: Array::new(),
            free_indices: Array::new(),
        }
    }

    pub fn get_uncasted(&self, user_handle: UserHandle<HandleRef>) -> Result<&Handle<HandleRef>> {
        self.array.get(user_handle.inner() as usize)
            .and_then(|obj| obj.as_ref())
            .ok_or(Error::NOT_FOUND)
    }

    pub fn get<T: HandleRef>(&self, user_handle: UserHandle<T>) -> Result<Handle<T>> {
        self.array.get(user_handle.inner() as usize)
            .and_then(|obj| obj.as_ref())
            .ok_or(Error::NOT_FOUND)
            .and_then(|handle| handle.cast())
    }

    /// This makes a copy of the supplied handle
    /// and inserts it into `self`.
    pub fn transfer_handle(&mut self, handle: Handle<HandleRef>) -> Result<UserHandle<HandleRef>> {
        if handle.rights().contains(HandleRights::TRANSFER) {
            self.allocate_handle_uncasted(handle)
        } else {
            Err(Error::ACCESS_DENIED)
        }
    }

    fn allocate_handle_uncasted(&mut self, handle: Handle<HandleRef>) -> Result<UserHandle<HandleRef>> {
        if let Some(index) = self.free_indices.pop() {
            debug_assert!(self.array[index].is_none());
            self.array[index] = Some(handle);
            Ok(UserHandle::<HandleRef>::new(index as u32))
        } else {
            self.array.push(Some(handle))?;
            Ok(UserHandle::<HandleRef>::new(self.array.len() as u32 - 1))
        }
    }

    fn allocate_handle<T: HandleRef>(&mut self, handle: Handle<T>) -> Result<UserHandle<T>> {
        if let Some(index) = self.free_indices.pop() {
            debug_assert!(self.array[index].is_none());
            self.array[index] = Some(handle.upcast());
            Ok(UserHandle::<T>::new(index as u32))
        } else {
            self.array.push(Some(handle.upcast()))?;
            Ok(UserHandle::<T>::new(self.array.len() as u32 - 1))
        }
    }

    pub fn allocate<T: HandleRef>(&mut self, refptr: Ref<T>, rights: HandleRights) -> Result<UserHandle<T>> {
        let handle = Handle::new(refptr, rights);
        self.allocate_handle(handle)
    }

    pub fn free_uncasted(&mut self, user_handle: UserHandle<HandleRef>) -> Result<Handle<HandleRef>> {
        let index = user_handle.inner() as usize;
        let handle = self.array.replace_at(index, None)
            .and_then(|opt| opt)
            .and_then(|handle| Some(handle))
            .ok_or(Error::NOT_FOUND)?;
        
        self.free_indices.push(index)?;
        Ok(handle)
    }

    pub fn free<T: HandleRef>(&mut self, user_handle: UserHandle<T>) -> Result<Handle<T>> {
        let index = user_handle.inner() as usize;
        let handle = self.array.replace_at(index, None)
            .and_then(|opt| opt)
            .and_then(|handle| Some(handle))
            .ok_or(Error::NOT_FOUND)?;
        
        self.free_indices.push(index)?;
        handle.cast()
    }

    pub fn duplicate_uncasted(&mut self, user_handle: UserHandle<HandleRef>, new_rights: HandleRights) -> Result<UserHandle<HandleRef>> {
        let dup = {
            let handle = self.get_uncasted(user_handle)?;
            handle.duplicate(new_rights)
                    .ok_or(Error::ACCESS_DENIED)
        }?;

        self.allocate_handle_uncasted(dup)
    }

    pub fn duplicate<T: HandleRef>(&mut self, user_handle: UserHandle<T>, new_rights: HandleRights) -> Result<UserHandle<T>> {
        let dup = {
            let handle = self.get(user_handle)?;
            handle.duplicate(new_rights)
                    .ok_or(Error::ACCESS_DENIED)
        }?;

        self.allocate_handle(dup)
    }
}
