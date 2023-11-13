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

thread_local! {
    static MEMORY_MANAGER: RefCell<MemoryManager<DefaultMemoryImpl>> = RefCell::new(
        MemoryManager::init(DefaultMemoryImpl::default())
    );

    static ID_COUNTER: RefCell<IdCell> = RefCell::new(
        IdCell::init(MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(0))), 0)
            .expect("Cannot create a counter")
    );

    static STORAGE: RefCell<StableBTreeMap<u64, Task, Memory>> =
        RefCell::new(StableBTreeMap::init(
            MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(1)))
    ));
}

#[derive(candid::CandidType, Serialize, Deserialize, Default)]
struct TaskPayload {
    title: String,
    description: String,
    due_date: u64,
}

#[ic_cdk::query]
fn get_task(id: u64) -> Result<Task, Error> {
    match _get_task(&id) {
        Some(task) => Ok(task),
        None => Err(Error::NotFound {
            msg: format!("a task with id={} not found", id),
        }),
    }
}

#[ic_cdk::update]
fn add_task(task: TaskPayload) -> Option<Task> {
    let id = ID_COUNTER
        .with(|counter| {
            let current_value = *counter.borrow().get();
            counter.borrow_mut().set(current_value + 1)
        })
        .expect("cannot increment id counter");
    let new_task = Task {
        id,
        title: task.title,
        description: task.description,
        due_date: task.due_date,
        completed: false,
        created_at: time(),
        updated_at: None,
    };
    do_insert_task(&new_task);
    Some(new_task)
}

#[ic_cdk::update]
fn update_task(id: u64, payload: TaskPayload) -> Result<Task, Error> {
    match STORAGE.with(|service| service.borrow().get(&id)) {
        Some(mut task) => {
            task.title = payload.title;
            task.description = payload.description;
            task.due_date = payload.due_date;
            task.updated_at = Some(time());
            do_insert_task(&task);
            Ok(task)
        }
        None => Err(Error::NotFound {
            msg: format!("couldn't update a task with id={}. task not found", id),
        }),
    }
}

// helper method to perform task insert.
fn do_insert_task(task: &Task) {
    STORAGE.with(|service| service.borrow_mut().insert(task.id, task.clone()));
}

#[ic_cdk::update]
fn delete_task(id: u64) -> Result<Task, Error> {
    match STORAGE.with(|service| service.borrow_mut().remove(&id)) {
        Some(task) => Ok(task),
        None => Err(Error::NotFound {
            msg: format!("couldn't delete a task with id={}. task not found.", id),
        }),
    }
}

#[derive(candid::CandidType, Deserialize, Serialize)]
enum Error {
    NotFound { msg: String },
}

// a helper method to get a task by id. used in get_task/update_task
fn _get_task(id: &u64) -> Option<Task> {
    STORAGE.with(|service| service.borrow().get(id))
}

// need this to generate candid
ic_cdk::export_candid!();

