# Example usage of SQLX crate
**1. install sqlx-cli (Command Line Interface)** 
- ```cargo install sqlx-cli```

**2. compose postgres container**
- install docker or skip this step and create your own postgres server 
- ```docker-compose up -d``` (don't forget to ```docker-compose down``` at the end)

**3. set DATABASE_URL enviroment variable (needed for sqlx-cli commands)**
- PWS:  ```$env:DATABASE_URL="sqlite:todos.db"```
- CMD:  ```set DATABASE_URL="postgres://postgres:password@localhost/todos"```
- Bash: ```export DATABASE_URL="example:example"```

**4. create DB through sqlx-cli**
- ```sqlx database create```
- create for both SQLite and Postgres (change database_url env variable and run again)

**5. you can comment one of DB_URL constants in src file to disable that DBMS**

**6. cargo run**
- use ```cargo run -- help``` to see subcommands
****
sadly we can't use the sqlx ```query!``` macro because of the fact, that we want to utilize both Postgres and SQLite DBMS means that the program wouldn't compile as all macro queries are checked at compile time and we can set these check just for one type of DBMS so the other type will always throw errors.