# ICP task management app

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
    title = "Complete you dacade task";
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
  	title = "Complete you dacade task (Updated!)";
    description = "Make sure you do it well and diligently! (Updated!)";
    due_date = "14-11-2023";
    completed = true;
})'
```

### Delete task by task's ID

```bash
dfx canister call icp_task_management_app_backend delete_task '(0)'
```
