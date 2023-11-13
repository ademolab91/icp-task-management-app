#[macro_use]
extern crate serde;
use candid::{Decode, Encode};
use ic_cdk::api::time;
use ic_stable_structures::memory_manager::{MemoryId, MemoryManager, VirtualMemory};
use ic_stable_structures::{BoundedStorable, Cell, DefaultMemoryImpl, StableBTreeMap, Storable};
use std::{borrow::Cow, cell::RefCell};

type Memory = VirtualMemory<DefaultMemoryImpl>;
type IdCell = Cell<u64, Memory>;

#[derive(candid::CandidType, Clone, Serialize, Deserialize, Default)]
struct Task {
    id: u64,
    title: String,
    description: String,
    due_date: u64,
    completed: bool,
    created_at: u64,
    updated_at: Option<u64>,
}

// a trait that must be implemented for a struct that is stored in a stable struct
impl Storable for Task {
    fn to_bytes(&self) -> std::borrow::Cow<[u8]> {
        Cow::Owned(Encode!(self).unwrap())
    }

    fn from_bytes(bytes: std::borrow::Cow<[u8]>) -> Self {
        Decode!(bytes.as_ref(), Self).unwrap()
    }
}

// another trait that must be implemented for a struct that is stored in a stable struct
impl BoundedStorable for Task {
    const MAX_SIZE: u32 = 1024;
    const IS_FIXED_SIZE: bool = false;
}

lazy_static! {
    static ref MEMORY_MANAGER: RefCell<MemoryManager<DefaultMemoryImpl>> =
        RefCell::new(MemoryManager::init(DefaultMemoryImpl::default()));
    static ref ID_GENERATOR: RefCell<u64> = RefCell::new(0);
    static ref STORAGE: RefCell<StableBTreeMap<u64, Task, Memory>> =
        RefCell::new(StableBTreeMap::init(MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(1)))));
}

#[derive(candid::CandidType, Serialize, Deserialize, Default)]
struct TaskPayload {
    title: String,
    description: String,
    due_date: u64,
}

#[ic_cdk::query]
fn get_task(id: u64) -> Result<Task, Error> {
    match STORAGE.with(|service| service.borrow().get(&id)) {
        Some(task) => Ok(task),
        None => Err(Error::NotFound {
            msg: format!("a task with id={} not found", id),
        }),
    }
}

#[ic_cdk::update]
fn add_task(task: TaskPayload) -> Option<Task> {
    let id = ID_GENERATOR.with(|generator| {
        let current_id = *generator.borrow();
        *generator.borrow_mut() = current_id + 1;
        current_id
    });

    if task.title.is_empty() || task.description.is_empty() {
        return None;
    }

    let new_task = Task {
        id,
        title: task.title,
        description: task.description,
        due_date: task.due_date,
        completed: false,
        created_at: time(),
        updated_at: None,
    };

    STORAGE.with(|service| service.borrow_mut().insert(new_task.id, new_task.clone()));
    Some(new_task)
}

#[ic_cdk::update]
fn update_task(id: u64, payload: TaskPayload) -> Result<Task, Error> {
    match STORAGE.with(|service| service.borrow().get(&id)) {
        Some(mut task) => {
            if !payload.title.is_empty() {
                task.title = payload.title;
            }

            if !payload.description.is_empty() {
                task.description = payload.description;
            }

            if task.due_date != payload.due_date {
                task.due_date = payload.due_date;
            }

            task.updated
