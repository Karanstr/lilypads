use std::ops::Deref;

#[derive(Debug)]
pub enum AccessError {
    OutOfBoundsMemory(usize),
    ProtectedMemory(usize),
    FreeMemory(usize),
    InvalidRequest,
}


mod reference_management {

    #[derive(PartialEq)]
    pub enum ReferenceStatus {
        Fine(usize),
        Dangling,
    }
    
    pub struct ReferenceWrapper<T> {
        pub data : T,
        ref_count:usize,
    }
    
    impl<T> ReferenceWrapper<T> {
    
        pub fn new(data:T) -> Self {
            Self {
                data,
                ref_count : 0,
            }
        }
    
        pub fn modify_ref(&mut self, delta:isize) -> Result<ReferenceStatus, super::AccessError> {
            if delta < 0 {
                match self.ref_count.checked_sub(delta.abs() as usize) {
                    Some(zero) if zero == 0 => {
                        self.ref_count = 0;
                        Result::Ok(ReferenceStatus::Dangling)
                    },
                    Some(new_ref) => {
                        self.ref_count = new_ref;
                        Result::Ok(ReferenceStatus::Fine(self.ref_count))
                    },
                    None => Result::Err( super::AccessError::InvalidRequest )
                }     
            } else {
                self.ref_count += delta as usize;
                Result::Ok(ReferenceStatus::Fine(self.ref_count))
            }
        }
    
        pub fn status(&self) -> ReferenceStatus {
            if self.ref_count == 0 {
                ReferenceStatus::Dangling
            } else {
                ReferenceStatus::Fine(self.ref_count)
            }
        }
    
    }

}


#[derive(Debug, PartialEq, PartialOrd, Clone, Copy, Hash, Eq)]
pub struct Index(pub usize);

impl Deref for Index {
    type Target = usize;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}


use reference_management::*;

pub struct FakeHeap<T:Clone> {
    memory:Vec< Option< ReferenceWrapper< T > > >,

    free_indexes : Vec<Index>,
    protected_indexes : Vec<Index>,
}

impl<T:Clone> FakeHeap<T> {

    pub fn new() -> Self {
        Self {
            memory : Vec::new(),
            free_indexes : Vec::new(),
            protected_indexes : Vec::new(),
        }
    }


    fn upper_bound(&self) -> Index {
        Index(self.memory.len())
    }

    fn last_index(&self) -> Index {
        Index(self.memory.len() - 1)
    }

    fn is_protected(&self, index:Index) -> bool {
        self.protected_indexes.contains(&index)
    }

    //Not sure how to correctly implement this visibility-wise, stand by
    //Will only free leaked memory which isn't considered protected
    fn remove_memory_leaks(&mut self) {
        for cell in 0 .. self.memory.len() {
            let index = Index(cell);
            if let Ok(wrapper) = self.mut_wrapper(index) {
                if let ReferenceStatus::Dangling = wrapper.status() {
                    self.free_index(index);
                }
            }
        }
    }

    fn mut_wrapper(&mut self, index:Index) -> Result<&mut ReferenceWrapper<T>, AccessError> {
        match index {
            bad_index if index >= self.upper_bound() => Err( AccessError::OutOfBoundsMemory(*bad_index) ),
            protected_index if self.is_protected(protected_index) => Err( AccessError::ProtectedMemory(*protected_index) ),
            index => match &mut self.memory[*index] {
                None => Err( AccessError::FreeMemory(*index) ),
                Some(tracker) => Ok(tracker)
            }
        }
    }

    fn wrapper(&self, index:Index) -> Result<&ReferenceWrapper<T>, AccessError> {
        match index {
            bad_index if index >= self.upper_bound() => Err( AccessError::OutOfBoundsMemory(*bad_index) ),
            index => match &self.memory[*index] {
                None => Err( AccessError::FreeMemory(*index) ),
                Some(tracker) => Ok(tracker)
            }
        }
    }

    fn free_index(&mut self, index:Index) -> Option<T> {
        let data = match self.mut_wrapper(index) {
            Err( error ) => { dbg!(error); panic!() },
            Ok(wrapper) => wrapper.data.clone()
        };
        self.memory[*index] = None;
        self.free_indexes.push(index);
        Some(data)
    }

    fn reserve_index(&mut self) -> Index {
        match self.free_indexes.pop() {
            Some(index) => index,
            None => {
                self.memory.push(None);
                self.last_index()
            }
        }        
    }

    fn reserve_protected(&mut self) -> Index {
        let index = match self.free_indexes.pop() {
            Some(index) => index,
            None => {
                self.memory.push(None);
                self.last_index()
            }
        };
        self.protected_indexes.push(index);
        index
    }

    //Not sure how to correctly implement this visibility-wise, stand by
    fn remove_protection(&mut self, index:Index) {
        if self.is_protected(index) {
            for protected in 0 .. self.protected_indexes.len() {
                if self.protected_indexes[protected] == index {
                    self.protected_indexes.remove(protected);
                    break
                }
            }
        }
    }


    pub fn data(&self, index:Index) -> Result<&T, AccessError> {
        Ok(&self.wrapper(index)?.data)
    }

    pub fn add_ref(&mut self, index:Index) -> Result<(), AccessError> {
        self.mut_wrapper(index)?.modify_ref(1)?;
        Ok(())
    }

    pub fn remove_ref(&mut self, index:Index) -> Result<Option<T>, AccessError> {
        if let ReferenceStatus::Dangling = self.mut_wrapper(index)?.modify_ref(-1)? {
            match self.free_index(index) {
                Some(data) => Ok( Some(data) ),
                None => Ok(None)
            }
        } else { Ok(None) }
    }

    //It is the responsibility of whatever calls push to take the index and call add_ref with it
    //Failure to do this will lead to dangling data and leak memory :(
    pub fn push(&mut self, data:T, protected:bool) -> Index {
        let index = if protected { self.reserve_protected() } else { self.reserve_index() };
        self.memory[*index] = Some( ReferenceWrapper::new(data) );
        index
    }

}