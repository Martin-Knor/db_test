use sqlx::{postgres::PgPool, sqlite::SqlitePool, Row, Executor};
use std::sync::Arc;
use structopt::StructOpt;
use async_trait::async_trait;

const DATABASE_URL_SQL: &str = "sqlite:todos.db";
const DATABASE_URL_POSTGRES: &str = "postgres://postgres:password@localhost/todos";

#[derive(StructOpt)]
struct Args {
    #[structopt(subcommand)]
    cmd: Option<Command>,
}

#[derive(StructOpt)]
enum Command {
    Add { description: String },
    Done { id: i64 },
    Clear,
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> anyhow::Result<()> {
    // Parse command line arguments
    let args = Args::from_args_safe()?;

    if DATABASE_URL_SQL.starts_with("sqlite:") {
        let pool = SqlitePool::connect(DATABASE_URL_SQL).await?;
        let sqlite_db = SqliteDBTraits::new(pool);

        // Run the CREATE TABLE query
        sqlite_db.create_table().await?;

        handle_command(&args, &sqlite_db).await.expect("panic");
    }
    if DATABASE_URL_POSTGRES.starts_with("postgres:") {
        let pool = PgPool::connect(DATABASE_URL_POSTGRES).await?;
        let postgres_db = PostgresDBTraits::new(pool);

        // Run the CREATE TABLE query
        postgres_db.create_table().await?;

        handle_command(&args, &postgres_db).await.expect("panic");
    }
    if !(DATABASE_URL_SQL.starts_with("sqlite:") || DATABASE_URL_POSTGRES.starts_with("postgres:")) {
        return Err(anyhow::anyhow!("Unsupported database type"));
    }

    Ok(())
}

async fn handle_command(
    args: &Args,
    database: &impl DBTraits,
) -> anyhow::Result<()> {
    match &args.cmd {
        Some(Command::Add { description }) => {
            println!("Adding new todo with description '{}'", &description);
            let todo_id = database.add_todo(description.clone()).await?;
            println!("Added new todo with id {todo_id}");
        }
        Some(Command::Done { id }) => {
            println!("Marking todo {id} as done");
            if database.complete_todo(*id).await? {
                println!("Todo {id} is marked as done");
            } else {
                println!("Invalid id {id}");
            }
        }
        Some(Command::Clear) => {
            println!("Clearing TODOs");
            database.clear_todos().await?;
            println!("TODOs were cleared");
        }
        None => {
            println!("Printing list of all todos");
            database.list_todos().await?;
        }
    }

    Ok(())
}

// database interface
#[async_trait]
pub trait DBTraits {
    async fn create_table(&self) -> anyhow::Result<()>;
    async fn add_todo(&self, description: String) -> anyhow::Result<i64>;
    async fn complete_todo(&self, id: i64) -> anyhow::Result<bool>;
    async fn clear_todos(&self) -> anyhow::Result<()>;
    async fn list_todos(&self) -> anyhow::Result<()>;
}

struct SqliteDBTraits {
    sqlite_pool: Arc<SqlitePool>,
}

impl SqliteDBTraits {
    fn new(sqlite_pool: SqlitePool) -> Self {
        Self {
            sqlite_pool: Arc::new(sqlite_pool),
        }
    }
}

struct PostgresDBTraits {
    pg_pool: Arc<PgPool>,
}

impl PostgresDBTraits {
    fn new(pg_pool: PgPool) -> Self {
        Self {
            pg_pool: Arc::new(pg_pool),
        }
    }
}


/*-----------------------------------*/
/*           sqlite methods          */
/*-----------------------------------*/
#[async_trait]
impl DBTraits for SqliteDBTraits {
    async fn create_table(&self) -> anyhow::Result<()> {
        self.sqlite_pool.execute(
            r#"
            CREATE TABLE IF NOT EXISTS todos (
                id INTEGER PRIMARY KEY NOT NULL,
                description TEXT NOT NULL,
                done BOOLEAN NOT NULL DEFAULT 0
            )
            "#,
        )
        .await?;
        Ok(())
    }

    async fn add_todo(&self, description: String) -> anyhow::Result<i64> {
        // Insert the task, then obtain the ID of this row
        let id = sqlx::query(
            r#"
    INSERT INTO todos (description)
    VALUES (?1)
            "#
        )
        .bind(description)
        .execute(&*self.sqlite_pool)
        .await?
        .last_insert_rowid();

        Ok(id)
    }

    async fn complete_todo(&self, id: i64) -> anyhow::Result<bool> {
        let rows_affected = sqlx::query(
            r#"
UPDATE todos
SET done = TRUE
WHERE id = $1
            "#
        )
        .bind(id)
        .execute(&*self.sqlite_pool)
        .await?
        .rows_affected();

        Ok(rows_affected > 0)
    }

    async fn clear_todos(&self) -> anyhow::Result<()> {
        sqlx::query(
            r#"
    DELETE FROM todos
            "#,
        )
        .fetch_all(&*self.sqlite_pool)
        .await?;

        Ok(())
    }

    async fn list_todos(&self) -> anyhow::Result<()> {
        let recs = sqlx::query(
            r#"
SELECT id, description, done
FROM todos
ORDER BY id
            "#
        )
        .fetch_all(&*self.sqlite_pool)
        .await?;

        for rec in recs {
            let id: i64 = rec.get("id");
            let description: String = rec.get("description");
            let done: bool = rec.get("done");

            println!(
                "- [{}] {}: {}",
                if done { "x" } else { " " },
                id,
                description,
            );
        }

        Ok(())
    }
}



/*-----------------------------------*/
/*         postgres methods          */
/*-----------------------------------*/

#[async_trait]
impl DBTraits for PostgresDBTraits {
    async fn create_table(&self) -> anyhow::Result<()> {
        self.pg_pool.execute(
            r#"
            CREATE TABLE IF NOT EXISTS todos (
                id SERIAL PRIMARY KEY,
                description TEXT NOT NULL,
                done BOOLEAN NOT NULL DEFAULT FALSE
            )
            "#,
        )
        .await?;
        Ok(())
    }

    async fn add_todo(&self, description: String) -> anyhow::Result<i64> {
        // Insert and return the newly inserted row's ID
        let rec = sqlx::query(
            r#"
INSERT INTO todos (description)
VALUES ($1)
RETURNING id
            "#
        )
        .bind(description)
        .fetch_one(&*self.pg_pool)
        .await?;

        let id: i64 = rec.get("id");
        Ok(id)
    }

    async fn complete_todo(&self, id: i64) -> anyhow::Result<bool> {
        let rows_affected = sqlx::query(
            r#"
UPDATE todos
SET done = TRUE
WHERE id = $1
            "#
        )
        .bind(id)
        .execute(&*self.pg_pool)
        .await?
        .rows_affected();

        Ok(rows_affected > 0)
    }

    async fn clear_todos(&self) -> anyhow::Result<()> {
        sqlx::query(
            r#"
    DELETE FROM todos
            "#,
        )
        .fetch_all(&*self.pg_pool)
        .await?;

        Ok(())
    }

    async fn list_todos(&self) -> anyhow::Result<()> {
        let recs = sqlx::query(
            r#"
SELECT id, description, done
FROM todos
ORDER BY id
            "#
        )
        .fetch_all(&*self.pg_pool)
        .await?;

        for rec in recs {
            let id: i64 = rec.get("id");
            let description: String = rec.get("description");
            let done: bool = rec.get("done");

            println!(
                "- [{}] {}: {}",
                if done { "x" } else { " " },
                id,
                description,
            );
        }

        Ok(())
    }
}