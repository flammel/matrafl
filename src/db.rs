use std::str;

use argon2::{
    password_hash::{rand_core::OsRng, PasswordHasher, SaltString},
    Argon2,
};
use rand::Rng;
use serde_json::json;
use sha2::{Digest, Sha256};
use sqlx::SqlitePool;
use uuid::Uuid;

pub const SESSION_DAYS: i64 = 7;

#[derive(Debug, Clone)]
pub struct Db {
    db_pool: SqlitePool,
}

#[derive(sqlx::FromRow)]
pub struct UserWithPasswordHash {
    pub id: UserId,
    pub password_hash: String,
}

#[derive(Debug, PartialEq, Eq, Clone, sqlx::Decode, sqlx::Encode)]
pub struct UserId(pub String);

impl sqlx::Type<sqlx::Sqlite> for UserId {
    fn type_info() -> <sqlx::Sqlite as sqlx::Database>::TypeInfo {
        <String as sqlx::Type<sqlx::Sqlite>>::type_info()
    }

    fn compatible(ty: &<sqlx::Sqlite as sqlx::Database>::TypeInfo) -> bool {
        <String as sqlx::Type<sqlx::Sqlite>>::compatible(ty)
    }
}

#[derive(sqlx::FromRow)]
pub struct WeightWithDate {
    pub id: String,
    pub user_id: UserId,
    pub weight: f64,
    pub measured_at: chrono::NaiveDate,
}

#[derive(sqlx::FromRow)]
pub struct Food {
    pub id: String,
    pub user_id: UserId,
    pub name: String,
    pub kcal: f64,
    pub fat: f64,
    pub carbs: f64,
    pub protein: f64,
    pub hidden_at: Option<chrono::NaiveDateTime>,
    pub starred_at: Option<chrono::NaiveDateTime>,
}

#[derive(sqlx::FromRow)]
pub struct Recipe {
    pub id: String,
    pub user_id: UserId,
    pub name: String,
    pub quantity: f64,
    pub kcal: f64,
    pub fat: f64,
    pub carbs: f64,
    pub protein: f64,
    pub hidden_at: Option<chrono::NaiveDateTime>,
    pub starred_at: Option<chrono::NaiveDateTime>,
}

pub enum ConsumptionFilter {
    None,
    ConsumedAt(chrono::NaiveDate),
    FoodId(String),
    RecipeId(String),
}

#[derive(sqlx::FromRow)]
pub struct Consumption {
    pub id: String,
    pub user_id: UserId,
    pub consumable_id: String,
    pub consumable_name: String,
    pub quantity: f64,
    pub kcal: f64,
    pub fat: f64,
    pub carbs: f64,
    pub protein: f64,
    pub consumed_at: chrono::NaiveDate,
}

#[derive(sqlx::FromRow)]
pub struct Ingredient {
    pub id: String,
    pub user_id: UserId,
    pub recipe_id: String,
    pub food_id: String,
    pub food_name: String,
    pub quantity: f64,
    pub kcal: f64,
    pub fat: f64,
    pub carbs: f64,
    pub protein: f64,
}

pub struct Macros {
    pub kcal: f64,
    pub fat: f64,
    pub carbs: f64,
    pub protein: f64,
}

#[derive(PartialEq, Eq)]
pub enum ConsumableType {
    Food,
    Recipe,
}

impl ConsumableType {
    pub fn as_str(&self) -> &str {
        match self {
            ConsumableType::Food => "food",
            ConsumableType::Recipe => "recipe",
        }
    }
}

pub struct Consumable {
    pub ctype: ConsumableType,
    pub id: String,
    pub name: String,
    pub is_starred: bool,
    created_at: chrono::NaiveDateTime,
    last_consumed_at: Option<chrono::NaiveDate>,
    consumed_count: Option<i64>,
}

impl Db {
    pub fn new(db_pool: SqlitePool) -> Self {
        Self { db_pool }
    }

    pub async fn get_weights(&self, user_id: &UserId) -> Result<Vec<WeightWithDate>, sqlx::Error> {
        sqlx::query_as::<_, WeightWithDate>(
            r#"
            SELECT
                id,
                user_id,
                weight,
                date(measured_at) as measured_at
            FROM
                weights
            WHERE
                user_id = ?
            ORDER BY
                measured_at DESC
            "#,
        )
        .bind(user_id)
        .fetch_all(&self.db_pool)
        .await
    }

    pub async fn get_weight(&self, id: &str) -> Result<WeightWithDate, sqlx::Error> {
        sqlx::query_as::<_, WeightWithDate>(
            r#"
            SELECT
                id,
                user_id,
                weight,
                date(measured_at) as measured_at
            FROM
                weights
            WHERE
                id = ?
            "#,
        )
        .bind(id)
        .fetch_one(&self.db_pool)
        .await
    }

    pub async fn get_weight_by_date(
        &self,
        user_id: &UserId,
        measured_at: &chrono::NaiveDate,
    ) -> Result<Option<WeightWithDate>, sqlx::Error> {
        sqlx::query_as::<_, WeightWithDate>(
            r#"
            SELECT
                id,
                user_id,
                weight,
                date(measured_at) as measured_at
            FROM
                weights
            WHERE
                user_id = ?
                AND date(measured_at) = date(?)
            "#,
        )
        .bind(user_id)
        .bind(measured_at)
        .fetch_optional(&self.db_pool)
        .await
    }

    pub async fn add_weight(
        &self,
        user_id: &UserId,
        weight: f64,
        measured_at: &chrono::NaiveDate,
    ) -> Result<(), sqlx::Error> {
        let now = chrono::Utc::now();
        sqlx::query("INSERT INTO weights (id, user_id, weight, measured_at, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?)")
            .bind(Uuid::new_v4().to_string())
            .bind(user_id)
            .bind(weight)
            .bind(measured_at)
            .bind(now)
            .bind(now)
            .execute(&self.db_pool)
            .await?;
        Ok(())
    }

    pub async fn update_weight(
        &self,
        id: &str,
        weight: f64,
        measured_at: &chrono::NaiveDate,
    ) -> Result<(), sqlx::Error> {
        let now = chrono::Utc::now();
        sqlx::query("UPDATE weights SET weight = ?, measured_at = ?, updated_at = ? WHERE id = ?")
            .bind(weight)
            .bind(measured_at)
            .bind(now)
            .bind(id)
            .execute(&self.db_pool)
            .await?;
        Ok(())
    }

    pub async fn delete_weight(&self, id: &str) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM weights WHERE id = ?")
            .bind(id)
            .execute(&self.db_pool)
            .await?;
        Ok(())
    }

    pub async fn get_foods(&self, user_id: &UserId) -> Result<Vec<Food>, sqlx::Error> {
        sqlx::query_as::<_, Food>(
            "SELECT id, user_id, name, kcal, fat, carbs, protein, hidden_at, starred_at FROM foods WHERE user_id = ? ORDER BY updated_at DESC",
        )
        .bind(user_id)
        .fetch_all(&self.db_pool)
        .await
    }

    pub async fn get_food(&self, id: &str) -> Result<Food, sqlx::Error> {
        sqlx::query_as::<_, Food>(
            "SELECT id, user_id, name, kcal, fat, carbs, protein, hidden_at, starred_at FROM foods WHERE id = ?",
        )
        .bind(id)
        .fetch_one(&self.db_pool)
        .await
    }

    pub async fn add_food(
        &self,
        user_id: &UserId,
        name: &str,
        macros: Macros,
        hidden: bool,
        starred: bool,
    ) -> Result<(), sqlx::Error> {
        let now = chrono::Utc::now();
        sqlx::query("INSERT INTO foods (id, user_id, name, kcal, fat, carbs, protein, hidden_at, starred_at, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)")
            .bind(Uuid::new_v4().to_string())
            .bind(user_id)
            .bind(name)
            .bind(macros.kcal)
            .bind(macros.fat)
            .bind(macros.carbs)
            .bind(macros.protein)
            .bind(hidden.then_some(now))
            .bind(starred.then_some(now))
            .bind(now)
            .bind(now)
            .execute(&self.db_pool)
            .await?;
        Ok(())
    }

    pub async fn update_food(
        &self,
        id: &str,
        name: &str,
        macros: Macros,
        hidden: bool,
        starred: bool,
    ) -> Result<(), sqlx::Error> {
        let now = chrono::Utc::now();
        sqlx::query("UPDATE foods SET name = ?, kcal = ?, fat = ?, carbs = ?, protein = ?, hidden_at = min(?, coalesce(hidden_at, datetime())), starred_at = min(?, coalesce(starred_at, datetime())), updated_at = ? WHERE id = ?")
        .bind(name)
        .bind(macros.kcal)
        .bind(macros.fat)
        .bind(macros.carbs)
        .bind(macros.protein)
        .bind(hidden.then_some(now))
        .bind(starred.then_some(now))
        .bind(now)
        .bind(id)
        .execute(&self.db_pool)
        .await?;
        Ok(())
    }

    pub async fn delete_food(&self, id: &str) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM foods WHERE id = ?")
            .bind(id)
            .execute(&self.db_pool)
            .await?;
        Ok(())
    }

    pub async fn get_recipes(&self, user_id: &UserId) -> Result<Vec<Recipe>, sqlx::Error> {
        sqlx::query_as::<_, Recipe>(
            "
            SELECT
                r.id,
                r.user_id,
                r.name,
                r.quantity,
                sum(f.kcal * i.quantity) as kcal,
                sum(f.fat * i.quantity) as fat,
                sum(f.carbs * i.quantity) as carbs,
                sum(f.protein * i.quantity) as protein,
                r.hidden_at,
                r.starred_at
            FROM
                recipes r
            LEFT JOIN
                ingredients i
                    ON
                        r.id = i.recipe_id
            LEFT JOIN
                foods f
                    ON
                        i.food_id = f.id
            WHERE
                r.user_id = ?
            GROUP BY
                r.id
            ORDER BY
                r.updated_at DESC
            ",
        )
        .bind(user_id)
        .fetch_all(&self.db_pool)
        .await
    }

    pub async fn get_recipe(&self, id: &str) -> Result<Recipe, sqlx::Error> {
        sqlx::query_as::<_, Recipe>(
            "
            SELECT
                r.id,
                r.user_id,
                r.name,
                r.quantity,
                sum(f.kcal * i.quantity) as kcal,
                sum(f.fat * i.quantity) as fat,
                sum(f.carbs * i.quantity) as carbs,
                sum(f.protein * i.quantity) as protein,
                r.hidden_at,
                r.starred_at
            FROM
                recipes r
            LEFT JOIN
                ingredients i
                    ON
                        r.id = i.recipe_id
            LEFT JOIN
                foods f
                    ON
                        i.food_id = f.id
            WHERE
                r.id = ?
            GROUP BY
                r.id
            ORDER BY
                r.updated_at DESC
            ",
        )
        .bind(id)
        .fetch_one(&self.db_pool)
        .await
    }

    pub async fn add_recipe(
        &self,
        user_id: &UserId,
        name: &str,
        quantity: f64,
        hidden: bool,
        starred: bool,
    ) -> Result<(), sqlx::Error> {
        let now = chrono::Utc::now();
        sqlx::query("
            INSERT INTO recipes (id, user_id, name, quantity, hidden_at, starred_at, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?)
        ")
            .bind(Uuid::new_v4().to_string())
            .bind(user_id)
            .bind(name)
            .bind(quantity)
            .bind(hidden.then_some(now))
            .bind(starred.then_some(now))
            .bind(now)
            .bind(now)
            .execute(&self.db_pool)
            .await?;
        Ok(())
    }

    pub async fn update_recipe(
        &self,
        id: &str,
        name: &str,
        quantity: f64,
        hidden: bool,
        starred: bool,
    ) -> Result<(), sqlx::Error> {
        let now = chrono::Utc::now();
        sqlx::query("UPDATE recipes SET name = ?, quantity = ?, hidden_at = min(?, coalesce(hidden_at, datetime())), starred_at = min(?, coalesce(starred_at, datetime())), updated_at = ? WHERE id = ?")
        .bind(name)
        .bind(quantity)
        .bind(hidden.then_some(now))
        .bind(starred.then_some(now))
        .bind(now)
        .bind(id)
        .execute(&self.db_pool)
        .await?;
        Ok(())
    }

    pub async fn delete_recipe(&self, id: &str) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM recipes WHERE id = ?")
            .bind(id)
            .execute(&self.db_pool)
            .await?;
        Ok(())
    }

    pub async fn get_consumptions(
        &self,
        user_id: &UserId,
        filter: ConsumptionFilter,
    ) -> Result<Vec<Consumption>, sqlx::Error> {
        let filter_sql = match &filter {
            ConsumptionFilter::None => "?",
            ConsumptionFilter::ConsumedAt(_) => "date(c.consumed_at) = date(?)",
            ConsumptionFilter::FoodId(_) => "c.food_id = ?",
            ConsumptionFilter::RecipeId(_) => "c.recipe_id = ?",
        };
        let filter_value = match filter {
            ConsumptionFilter::None => Some("1".to_string()),
            ConsumptionFilter::ConsumedAt(date) => Some(date.to_string()),
            ConsumptionFilter::FoodId(food_id) => Some(food_id.clone()),
            ConsumptionFilter::RecipeId(recipe_id) => Some(recipe_id.clone()),
        };

        sqlx::query_as::<_, Consumption>(
            format!("
            SELECT
                c.id,
                c.user_id,
                coalesce(c.food_id, c.recipe_id) as consumable_id,
                c.quantity,
                date(c.consumed_at) as consumed_at,
                coalesce(f.name, r.name) as consumable_name,
                sum(coalesce(f.kcal * c.quantity, fi.kcal * i.quantity / r.quantity * c.quantity)) as kcal,
                sum(coalesce(f.fat * c.quantity, fi.fat * i.quantity / r.quantity * c.quantity)) as fat,
                sum(coalesce(f.carbs * c.quantity, fi.carbs * i.quantity / r.quantity * c.quantity)) as carbs,
                sum(coalesce(f.protein * c.quantity, fi.protein * i.quantity / r.quantity * c.quantity)) as protein
            FROM
                consumptions c
            LEFT JOIN
                foods f
                    ON
                        c.food_id = f.id
            LEFT JOIN
                recipes r
                    ON
                        c.recipe_id = r.id
            LEFT JOIN
                ingredients i
                    ON
                        r.id = i.recipe_id
            LEFT JOIN
                foods fi
                    ON
                        i.food_id = fi.id
            WHERE
                c.user_id = ?
                AND {filter_sql}
            GROUP BY
                c.id
            ORDER BY
                c.updated_at DESC
        ").as_str(),
        )
        .bind(user_id)
        .bind(filter_value)
        .fetch_all(&self.db_pool)
        .await
    }

    pub async fn get_consumption(&self, id: &str) -> Result<Consumption, sqlx::Error> {
        sqlx::query_as::<_, Consumption>(
            "
            SELECT
                c.id,
                c.user_id,
                coalesce(c.food_id, c.recipe_id) as consumable_id,
                c.quantity,
                date(c.consumed_at) as consumed_at,
                coalesce(f.name, r.name) as consumable_name,
                sum(coalesce(f.kcal * c.quantity, fi.kcal * i.quantity / r.quantity * c.quantity)) as kcal,
                sum(coalesce(f.fat * c.quantity, fi.fat * i.quantity / r.quantity * c.quantity)) as fat,
                sum(coalesce(f.carbs * c.quantity, fi.carbs * i.quantity / r.quantity * c.quantity)) as carbs,
                sum(coalesce(f.protein * c.quantity, fi.protein * i.quantity / r.quantity * c.quantity)) as protein
            FROM
                consumptions c
            LEFT JOIN
                foods f
                    ON
                        c.food_id = f.id
            LEFT JOIN
                recipes r
                    ON
                        c.recipe_id = r.id
            LEFT JOIN
                ingredients i
                    ON
                        r.id = i.recipe_id
            LEFT JOIN
                foods fi
                    ON
                        i.food_id = fi.id
            WHERE
                c.id = ?
            GROUP BY
                c.id
            ORDER BY
                c.updated_at DESC
        ",
        )
        .bind(id)
        .fetch_one(&self.db_pool)
        .await
    }

    pub async fn add_consumption(
        &self,
        user_id: &UserId,
        food_id: Option<&str>,
        recipe_id: Option<&str>,
        quantity: f64,
        consumed_at: &chrono::NaiveDate,
    ) -> Result<(), sqlx::Error> {
        let now = chrono::Utc::now();
        sqlx::query("INSERT INTO consumptions (id, user_id, food_id, recipe_id, quantity, consumed_at, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?)")
            .bind(Uuid::new_v4().to_string())
            .bind(user_id)
            .bind(food_id)
            .bind(recipe_id)
            .bind(quantity)
            .bind(consumed_at)
            .bind(now)
            .bind(now)
            .execute(&self.db_pool)
            .await?;
        Ok(())
    }

    pub async fn update_consumption(
        &self,
        id: &str,
        food_id: Option<&str>,
        recipe_id: Option<&str>,
        quantity: f64,
        consumed_at: &chrono::NaiveDate,
    ) -> Result<(), sqlx::Error> {
        let now = chrono::Utc::now();
        sqlx::query("UPDATE consumptions SET food_id = ?, recipe_id = ?, quantity = ?, consumed_at = ?, updated_at = ? WHERE id = ?")
        .bind(food_id)
        .bind(recipe_id)
        .bind(quantity)
            .bind(consumed_at)
            .bind(now)
            .bind(id)
            .execute(&self.db_pool)
            .await?;
        Ok(())
    }

    pub async fn delete_consumption(&self, id: &str) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM consumptions WHERE id = ?")
            .bind(id)
            .execute(&self.db_pool)
            .await?;
        Ok(())
    }

    pub async fn get_ingredients(&self, recipe_id: &str) -> Result<Vec<Ingredient>, sqlx::Error> {
        sqlx::query_as::<_, Ingredient>(
            r#"
            SELECT
                i.id,
                i.user_id,
                i.recipe_id,
                i.food_id,
                f.name as food_name,
                i.quantity,
                sum(f.kcal * i.quantity) as kcal,
                sum(f.fat * i.quantity) as fat,
                sum(f.carbs * i.quantity) as carbs,
                sum(f.protein * i.quantity) as protein
            FROM
                ingredients i
            LEFT JOIN
                foods f
                    ON
                        i.food_id = f.id
            WHERE
                i.recipe_id = ?
            GROUP BY
                i.id
            "#,
        )
        .bind(recipe_id)
        .fetch_all(&self.db_pool)
        .await
    }

    pub async fn get_ingredient(&self, id: &str) -> Result<Ingredient, sqlx::Error> {
        sqlx::query_as::<_, Ingredient>(
            r#"
            SELECT
                i.id,
                i.user_id,
                i.recipe_id,
                i.food_id,
                f.name as food_name,
                i.quantity,
                sum(f.kcal * i.quantity) as kcal,
                sum(f.fat * i.quantity) as fat,
                sum(f.carbs * i.quantity) as carbs,
                sum(f.protein * i.quantity) as protein
            FROM
                ingredients i
            LEFT JOIN
                foods f
                    ON
                        i.food_id = f.id
            WHERE
                i.id = ?
            GROUP BY
                i.id
            "#,
        )
        .bind(id)
        .fetch_one(&self.db_pool)
        .await
    }

    pub async fn add_ingredient(
        &self,
        user_id: &UserId,
        recipe_id: &str,
        food_id: &str,
        quantity: f64,
    ) -> Result<(), sqlx::Error> {
        let now = chrono::Utc::now();
        sqlx::query("INSERT INTO ingredients (id, user_id, recipe_id, food_id, quantity, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?)")
            .bind(Uuid::new_v4().to_string())
            .bind(user_id)
            .bind(recipe_id)
            .bind(food_id)
            .bind(quantity)
            .bind(now)
            .bind(now)
            .execute(&self.db_pool)
            .await?;
        Ok(())
    }

    pub async fn update_ingredient(
        &self,
        id: &str,
        food_id: &str,
        quantity: f64,
    ) -> Result<(), sqlx::Error> {
        let now = chrono::Utc::now();
        sqlx::query(
            "UPDATE ingredients SET food_id = ?, quantity = ?, updated_at = ? WHERE id = ?",
        )
        .bind(food_id)
        .bind(quantity)
        .bind(now)
        .bind(id)
        .execute(&self.db_pool)
        .await?;
        Ok(())
    }

    pub async fn delete_ingredient(&self, id: &str) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM ingredients WHERE id = ?")
            .bind(id)
            .execute(&self.db_pool)
            .await?;
        Ok(())
    }

    pub async fn get_user(
        &self,
        username: &str,
    ) -> Result<Option<UserWithPasswordHash>, sqlx::Error> {
        sqlx::query_as::<_, UserWithPasswordHash>(
            "SELECT id, password_hash FROM users WHERE username = ? LIMIT 1",
        )
        .bind(username)
        .fetch_optional(&self.db_pool)
        .await
    }

    pub async fn create_session(&self, user_id: &UserId) -> Result<String, sqlx::Error> {
        let session_id = rand::prelude::thread_rng().gen::<u128>().to_string();
        let hashed_id = base16ct::lower::encode_string(&Sha256::digest(&session_id));
        let now = chrono::Utc::now();
        sqlx::query("INSERT INTO sessions (id, user_id, created_at) VALUES (?, ?, ?)")
            .bind(&hashed_id)
            .bind(user_id)
            .bind(now)
            .execute(&self.db_pool)
            .await?;
        Ok(session_id)
    }

    pub async fn delete_session(&self, session_id: &str) -> Result<(), sqlx::Error> {
        let hashed_id = base16ct::lower::encode_string(&Sha256::digest(session_id));
        sqlx::query("DELETE FROM sessions WHERE id = ?")
            .bind(hashed_id)
            .execute(&self.db_pool)
            .await?;
        Ok(())
    }

    pub async fn delete_expired_sessions(&self) -> Result<(), sqlx::Error> {
        let created_at = chrono::Utc::now() - chrono::Duration::days(SESSION_DAYS);
        sqlx::query("DELETE FROM sessions WHERE created_at < ?")
            .bind(created_at)
            .execute(&self.db_pool)
            .await?;
        Ok(())
    }

    pub async fn get_session_user_id(
        &self,
        session_id: &str,
    ) -> Result<Option<String>, sqlx::Error> {
        let hashed_id = base16ct::lower::encode_string(&Sha256::digest(session_id));
        sqlx::query_scalar("SELECT user_id FROM sessions WHERE id = ? LIMIT 1")
            .bind(hashed_id)
            .fetch_optional(&self.db_pool)
            .await
    }

    pub async fn create_user(&self, username: &str, password: &str) -> Result<(), sqlx::Error> {
        let now = chrono::Utc::now();

        let salt = SaltString::generate(&mut OsRng);
        let password_hash = Argon2::default()
            .hash_password(password.as_bytes(), &salt)
            .unwrap()
            .to_string();

        sqlx::query(
            "INSERT INTO users (id, username, password_hash, created_at, updated_at) VALUES (?, ?, ?, ?, ?)",
        )
        .bind(Uuid::new_v4().to_string())
        .bind(username)
        .bind(password_hash)
        .bind(now)
        .bind(now)
        .fetch_optional(&self.db_pool)
        .await?;

        Ok(())
    }

    pub async fn export_data(&self, user_id: &UserId) -> Result<serde_json::Value, sqlx::Error> {
        #[derive(sqlx::FromRow, serde::Serialize)]
        struct WeightRow {
            id: String,
            weight: f64,
            measured_at: chrono::NaiveDate,
            created_at: chrono::NaiveDateTime,
            updated_at: chrono::NaiveDateTime,
        }

        #[derive(sqlx::FromRow, serde::Serialize)]
        struct FoodRow {
            id: String,
            name: String,
            kcal: f64,
            fat: f64,
            carbs: f64,
            protein: f64,
            hidden_at: Option<chrono::NaiveDateTime>,
            starred_at: Option<chrono::NaiveDateTime>,
            created_at: chrono::NaiveDateTime,
            updated_at: chrono::NaiveDateTime,
        }

        #[derive(sqlx::FromRow, serde::Serialize)]
        struct ConsumptionRow {
            id: String,
            food_id: Option<String>,
            recipe_id: Option<String>,
            quantity: f64,
            consumed_at: chrono::NaiveDate,
            created_at: chrono::NaiveDateTime,
            updated_at: chrono::NaiveDateTime,
        }

        #[derive(sqlx::FromRow, serde::Serialize)]
        struct RecipeRow {
            id: String,
            name: String,
            quantity: f64,
            hidden_at: Option<chrono::NaiveDateTime>,
            starred_at: Option<chrono::NaiveDateTime>,
            created_at: chrono::NaiveDateTime,
            updated_at: chrono::NaiveDateTime,
        }

        #[derive(sqlx::FromRow, serde::Serialize)]
        struct IngredientRow {
            id: String,
            recipe_id: String,
            food_id: String,
            quantity: f64,
            created_at: chrono::NaiveDateTime,
            updated_at: chrono::NaiveDateTime,
        }

        let weights = sqlx::query_as::<_, WeightRow>(
            "SELECT id, weight, date(measured_at) as measured_at, created_at, updated_at FROM weights WHERE user_id = ?",
        ).bind(user_id).fetch_all(&self.db_pool).await?;

        let foods = sqlx::query_as::<_, FoodRow>(
            "SELECT id, name, kcal, fat, carbs, protein,  hidden_at, starred_at, created_at, updated_at FROM foods WHERE user_id = ?",
        ).bind(user_id).fetch_all(&self.db_pool).await?;

        let consumptions = sqlx::query_as::<_, ConsumptionRow>(
            "SELECT id, food_id, recipe_id, quantity, date(consumed_at) as consumed_at, created_at, updated_at FROM consumptions WHERE user_id = ?",
        ).bind(user_id).fetch_all(&self.db_pool).await?;

        let recipes = sqlx::query_as::<_, RecipeRow>(
            "SELECT id, name, quantity, hidden_at, starred_at, created_at, updated_at FROM recipes WHERE user_id = ?",
        ).bind(user_id).fetch_all(&self.db_pool).await?;

        let ingredients = sqlx::query_as::<_, IngredientRow>(
            "SELECT id, recipe_id, food_id, quantity, created_at, updated_at FROM ingredients WHERE user_id = ?",
        ).bind(user_id).fetch_all(&self.db_pool).await?;

        Ok(json!({
            "user_id": user_id.0,
            "exported_at": chrono::Utc::now(),
            "weights": weights,
            "foods": foods,
            "consumptions": consumptions,
            "recipes": recipes,
            "ingredients": ingredients,
        }))
    }

    pub async fn get_consumables(&self, user_id: &UserId) -> Result<Vec<Consumable>, sqlx::Error> {
        let foods = sqlx::query_as::<
            _,
            (
                String,
                String,
                Option<chrono::NaiveDateTime>,
                chrono::NaiveDateTime,
                Option<chrono::NaiveDate>,
                Option<i64>,
            ),
        >(
            r#"
            SELECT
                f.id,
                f.name,
                f.starred_at,
                f.created_at,
                MAX(DATE(c.consumed_at)) as last_consumed_at,
                COUNT(c.id) as consumed_count
            FROM
                foods f
            LEFT JOIN
                consumptions c
                    ON
                        f.id = c.food_id
                        AND c.consumed_at > DATE('now', '-7 days')
            WHERE
                f.user_id = ?
                AND f.hidden_at IS NULL
            GROUP BY
                f.id
        "#,
        )
        .bind(user_id)
        .fetch_all(&self.db_pool)
        .await?;

        let recipes = sqlx::query_as::<
            _,
            (
                String,
                String,
                Option<chrono::NaiveDateTime>,
                chrono::NaiveDateTime,
                Option<chrono::NaiveDate>,
                Option<i64>,
            ),
        >(
            r#"
            SELECT
                r.id,
                r.name,
                r.starred_at,
                r.created_at,
                MAX(DATE(c.consumed_at)) as last_consumed_at,
                COUNT(c.id) as consumed_count
            FROM
                recipes r
            LEFT JOIN
                consumptions c
                    ON
                        r.id = c.recipe_id
                        AND c.consumed_at > DATE('now', '-7 days')
            WHERE
                r.user_id = ?
                AND r.hidden_at IS NULL
            GROUP BY
                r.id
        "#,
        )
        .bind(user_id)
        .fetch_all(&self.db_pool)
        .await?;

        let mut consumables: Vec<Consumable> = foods
            .into_iter()
            .map(|f| Consumable {
                ctype: ConsumableType::Food,
                id: f.0,
                name: f.1,
                is_starred: f.2.is_some(),
                created_at: f.3,
                last_consumed_at: f.4,
                consumed_count: f.5,
            })
            .chain(recipes.into_iter().map(|r| Consumable {
                ctype: ConsumableType::Recipe,
                id: r.0,
                name: r.1,
                is_starred: r.2.is_some(),
                created_at: r.3,
                last_consumed_at: r.4,
                consumed_count: r.5,
            }))
            .collect();

        consumables.sort();

        Ok(consumables)
    }
}

impl Ord for Consumable {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        sorting_points(other)
            .cmp(&sorting_points(self))
            .then(self.name.cmp(&other.name))
    }
}

impl PartialOrd for Consumable {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for Consumable {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for Consumable {}

fn sorting_points(consumable: &Consumable) -> i64 {
    let mut points = 0;
    if consumable.is_starred {
        points += 100;
    }
    if let Some(last_consumed_at) = consumable.last_consumed_at {
        let days_ago = chrono::Utc::now()
            .date_naive()
            .signed_duration_since(last_consumed_at)
            .num_days();
        points += 100 - days_ago;
    }
    if let Some(consumed_count) = consumable.consumed_count {
        points += consumed_count * 10;
    }
    if chrono::Utc::now()
        .naive_utc()
        .signed_duration_since(consumable.created_at)
        .num_minutes()
        < 5
    {
        points += 1000;
    }
    points
}
