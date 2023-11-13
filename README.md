# ICP task management app

This decentralized task management app on the Internet Computer network allows users to create, read, update, and delete tasks. Tasks have essential details like title, description, due date, and completion status. The smart contract ensures security and transparency in task management, leveraging the Internet Computer's stability and memory management features. Users can seamlessly organize and track tasks on the decentralized network.

## Features

1. Add new task
2. Delete a task
3. Update a task
4. Get a task

## Deploy Canister

```bash
dfx start --background --clean
npm run gen-deploy
```

## Commands

### Add new task

```bash
dfx canister call icp_task_management_app_backend add_task '(
  record {
    title = "Complete your dacade task";
    description = "Make sure you do it well and diligently!";
    due_date = "13-11-2023";
  }
)'
```

### Get task info by task's ID

```bash
dfx canister call icp_task_management_app_backend get_task '(0)'
```

### Update task info

```bash
dfx canister call icp_task_management_app_backend update_task '(0, record {
  	title = "Complete your dacade task (Updated!)";
    description = "Make sure you do it well and diligently! (Updated!)";
    due_date = "14-11-2023";
    completed = true;
})'
```

### Delete task by task's ID

```bash
dfx canister call icp_task_management_app_backend delete_task '(0)'
```
