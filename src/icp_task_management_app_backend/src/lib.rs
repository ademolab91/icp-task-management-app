#[macro_use]
extern crate serde;
use candid::{Decode, Encode};
use validator::Validate;
use ic_cdk::api::{time, caller};
use ic_stable_structures::memory_manager::{MemoryId, MemoryManager, VirtualMemory};
use ic_stable_structures::{BoundedStorable, Cell, DefaultMemoryImpl, StableBTreeMap, Storable};
use std::{borrow::Cow, cell::RefCell};

type Memory = VirtualMemory<DefaultMemoryImpl>;
type IdCell = Cell<u64, Memory>;

#[derive(candid::CandidType, Clone, Serialize, Deserialize, Default)]
struct Task {
    id: u64,
    owner: String,
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

#[derive(candid::CandidType, Serialize, Deserialize, Default, Validate)]
struct TaskPayload {
    #[validate(length(min = 3))]
    title: String,
    #[validate(length(min = 5))]
    description: String,
    due_date: u64,
}

// Function to fetch a task from the canister
#[ic_cdk::query]
fn get_task(id: u64) -> Result<Task, Error> {
    match _get_task(&id) {
        Some(task) => Ok(task),
        None => Err(Error::NotFound {
            msg: format!("a task with id={} not found", id),
        }),
    }
}

// Function to add a task to the canister
#[ic_cdk::update]
fn add_task(task: TaskPayload) -> Result<Task, Error> {
    // Validates payload
    let check_payload = _check_input(&task);
    // Returns an error if validations failed
    if check_payload.is_err(){
        return Err(check_payload.err().unwrap());
    }
    let id = ID_COUNTER
        .with(|counter| {
            let current_value = *counter.borrow().get();
            counter.borrow_mut().set(current_value + 1)
        })
        .expect("cannot increment id counter");
    let new_task = Task {
        id,
        owner: caller().to_string(),
        title: task.title,
        description: task.description,
        due_date: task.due_date,
        completed: false,
        created_at: time(),
        updated_at: None,
    };
    // save task
    do_insert_task(&new_task);
    Ok(new_task)
}

// Function to update a task in the canister
#[ic_cdk::update]
fn update_task(id: u64, payload: TaskPayload) -> Result<Task, Error> {
    match STORAGE.with(|service| service.borrow().get(&id)) {
        Some(mut task) => {
            // Validates whether caller is the owner of the task
            let check_if_owner = _check_if_owner(&task);
            if check_if_owner.is_err() {
                return Err(check_if_owner.err().unwrap())
            }
            // prevents misleading state change of the due_date field for completed tasks
            if task.completed {
                return Err(Error::AlreadyCompleted { msg: format!("Cannot update a completed task")})
            }
            // Validates payload
            let check_payload = _check_input(&payload);
            // Returns an error if validations failed
            if check_payload.is_err(){
                return Err(check_payload.err().unwrap());
            }
            task.title = payload.title;
            task.description = payload.description;
            task.due_date = payload.due_date;
            task.updated_at = Some(time());
            // save updated task
            do_insert_task(&task);
            Ok(task)
        }
        None => Err(Error::NotFound {
            msg: format!("couldn't update a task with id={}. task not found", id),
        }),
    }
}

// Function to mark a task as completed
#[ic_cdk::update]
fn complete_task(id: u64) -> Result<String, Error> {
    match STORAGE.with(|service| service.borrow().get(&id)) {
        Some(mut task) => {
            // Validates whether caller is the owner of the task
            let check_if_owner = _check_if_owner(&task);
            if check_if_owner.is_err() {
                return Err(check_if_owner.err().unwrap())
            }
            // prevents misleading state change of the updated_at field for completed tasks
            if task.completed {
                return Err(Error::AlreadyCompleted { msg: format!("Task is already completed")})
            }
            task.completed = true;
            task.updated_at = Some(time());
            // save updated task
            do_insert_task(&task);
            if task.due_date >= time() {
                return Ok(format!("Successfully completed task on time={} before due time={}", time(), task.due_date))
            }else{
                return Ok(format!("Failed to complete task on time={} before due time={}", time(), task.due_date))
            }
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

// Function to delete a canister from the canister
#[ic_cdk::update]
fn delete_task(id: u64) -> Result<Task, Error> {
    let task = _get_task(&id).expect(&format!("couldn't delete a task with id={}. task not found.", id));
    // Validates whether caller is the owner of the task
    let check_if_owner = _check_if_owner(&task);
    if check_if_owner.is_err() {
        return Err(check_if_owner.err().unwrap())
    }
    // delete task
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
    ValidationFailed { content: String},
    AuthenticationFailed {msg: String},
    AlreadyCompleted { msg: String}
}

// Helper function to check the input data of the payload
fn _check_input(payload: &TaskPayload) -> Result<(), Error> {
    let check_payload = payload.validate();
    if check_payload.is_err() {
        return Err(Error:: ValidationFailed{ content: check_payload.err().unwrap().to_string()})
    }else if payload.due_date < time() {
        return Err(Error::ValidationFailed { content: format!("Due date={} is currently set to be less than the current timestamp={}.", payload.due_date, time()) })
    }else{
        Ok(())
    }
}

// Helper function to check whether the caller is the owner of a task
fn _check_if_owner(task: &Task) -> Result<(), Error> {
    if task.owner.to_string() != caller().to_string(){
        return Err(Error:: AuthenticationFailed{ msg: format!("Caller={} isn't the owner of the task with id={}", caller(), task.id) })  
    }else{
        Ok(())
    }
}

// a helper method to get a task by id. used in get_task/update_task
fn _get_task(id: &u64) -> Option<Task> {
    STORAGE.with(|service| service.borrow().get(id))
}

// need this to generate candid
ic_cdk::export_candid!();

