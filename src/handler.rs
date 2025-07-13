use std::collections::HashMap;

use crate::db::{ConsumptionFilter, Macros, UserId, SESSION_DAYS};
use crate::html::AccountSummaryRow;
use crate::{html, redirect_to, AppError, AppState, AppUrl, Session};
use argon2::{Argon2, PasswordHash, PasswordVerifier};
use axum::body::Body;
use axum::extract::{Path, State};
use axum::http::{header, HeaderValue, StatusCode};
use axum::response::{Html, IntoResponse, Response};
use axum::Form;
use cookie::{Cookie, SameSite};
use maud::Markup;
use time::{Duration, OffsetDateTime};

pub fn authz(session: &Session, user_id: &UserId) -> bool {
    &session.user_id == user_id
}

pub async fn index(state: State<AppState>, session: Session) -> Result<Response, AppError> {
    let date = chrono::Utc::now().date_naive();

    days_read(state, session, Path(date)).await
}

pub async fn days_read(
    state: State<AppState>,
    session: Session,
    Path(date): Path<chrono::NaiveDate>,
) -> Result<Response, AppError> {
    let weight = state.db.get_weight_by_date(&session.user_id, &date).await?;
    let consumptions = state
        .db
        .get_consumptions(&session.user_id, ConsumptionFilter::ConsumedAt(date))
        .await?;
    let consumables = state.db.get_consumables(&session.user_id).await?;

    Ok(render_html(html::day_summary_page(
        date,
        weight,
        consumptions,
        consumables,
    )))
}

pub async fn weights_index(state: State<AppState>, session: Session) -> Result<Response, AppError> {
    let weights = state.db.get_weights(&session.user_id).await?;

    Ok(render_html(html::weights_page(weights)))
}

#[derive(Debug, serde::Deserialize)]
pub struct CreateWeightForm {
    weight: f64,
    measured_at: chrono::NaiveDate,
    redirect_to: Option<String>,
}

pub async fn weights_create(
    state: State<AppState>,
    session: Session,
    Form(form): Form<CreateWeightForm>,
) -> Result<Response, AppError> {
    state
        .db
        .add_weight(&session.user_id, form.weight, &form.measured_at)
        .await?;

    match form.redirect_to.map(|s| s.to_owned()).as_deref() {
        Some("DaySummary") => Ok(redirect_to(AppUrl::DaySummary(form.measured_at))),
        _ => Ok(redirect_to(AppUrl::Weights)),
    }
}

pub async fn weights_read(
    state: State<AppState>,
    session: Session,
    Path(id): Path<String>,
) -> Result<Response, AppError> {
    let weight = state.db.get_weight(&id).await?;

    if !authz(&session, &weight.user_id) {
        return Err(AppError::Forbidden);
    }

    Ok(render_html(html::weights_update_page(&weight)))
}

#[derive(Debug, serde::Deserialize)]
pub struct UpdateWeightForm {
    weight: f64,
    measured_at: chrono::NaiveDate,
}

pub async fn weights_update(
    state: State<AppState>,
    session: Session,
    Path(id): Path<String>,
    Form(form): Form<UpdateWeightForm>,
) -> Result<Response, AppError> {
    let weight = state.db.get_weight(&id).await?;

    if !authz(&session, &weight.user_id) {
        return Err(AppError::Forbidden);
    }

    state
        .db
        .update_weight(&id, form.weight, &form.measured_at)
        .await?;

    Ok(redirect_to(AppUrl::Weights))
}

pub async fn weights_delete(
    state: State<AppState>,
    session: Session,
    Path(id): Path<String>,
) -> Result<Response, AppError> {
    let weight = state.db.get_weight(&id).await?;

    if !authz(&session, &weight.user_id) {
        return Err(AppError::Forbidden);
    }

    state.db.delete_weight(&id).await?;

    Ok(redirect_to(AppUrl::Weights))
}

pub async fn foods_index(state: State<AppState>, session: Session) -> Result<Response, AppError> {
    let foods = state.db.get_foods(&session.user_id).await?;

    Ok(render_html(html::foods_page(foods)))
}

#[derive(Debug, serde::Deserialize)]
pub struct CreateFoodForm {
    name: String,
    kcal: f64,
    fat: f64,
    carbs: f64,
    protein: f64,
    hidden: Option<bool>,
    starred: Option<bool>,
}

pub async fn foods_create(
    state: State<AppState>,
    session: Session,
    Form(form): Form<CreateFoodForm>,
) -> Result<Response, AppError> {
    state
        .db
        .add_food(
            &session.user_id,
            &form.name,
            Macros {
                kcal: form.kcal,
                fat: form.fat,
                carbs: form.carbs,
                protein: form.protein,
            },
            form.hidden.is_some(),
            form.starred.is_some(),
        )
        .await?;

    Ok(redirect_to(AppUrl::Foods))
}

pub async fn foods_read(
    state: State<AppState>,
    session: Session,
    Path(id): Path<String>,
) -> Result<Response, AppError> {
    let food = state.db.get_food(&id).await?;

    if !authz(&session, &food.user_id) {
        return Err(AppError::Forbidden);
    }

    let consumptions = state
        .db
        .get_consumptions(&session.user_id, ConsumptionFilter::FoodId(id))
        .await?;

    Ok(render_html(html::foods_update_page(food, consumptions)))
}

#[derive(Debug, serde::Deserialize)]
pub struct UpdateFoodForm {
    name: String,
    kcal: f64,
    fat: f64,
    carbs: f64,
    protein: f64,
    hidden: Option<bool>,
    starred: Option<bool>,
}

pub async fn foods_update(
    state: State<AppState>,
    session: Session,
    Path(id): Path<String>,
    Form(form): Form<UpdateFoodForm>,
) -> Result<Response, AppError> {
    let food = state.db.get_food(&id).await?;

    if !authz(&session, &food.user_id) {
        return Err(AppError::Forbidden);
    }

    state
        .db
        .update_food(
            &id,
            &form.name,
            Macros {
                kcal: form.kcal,
                fat: form.fat,
                carbs: form.carbs,
                protein: form.protein,
            },
            form.hidden.is_some(),
            form.starred.is_some(),
        )
        .await?;

    Ok(redirect_to(AppUrl::Foods))
}

pub async fn foods_delete(
    state: State<AppState>,
    session: Session,
    Path(id): Path<String>,
) -> Result<Response, AppError> {
    let food = state.db.get_food(&id).await?;

    if !authz(&session, &food.user_id) {
        return Err(AppError::Forbidden);
    }

    state.db.delete_food(&id).await?;

    Ok(redirect_to(AppUrl::Foods))
}

#[derive(Debug, serde::Deserialize)]
pub struct CreateConsumptionForm {
    consumable_id: String,
    consumable_type: String,
    quantity: f64,
    consumed_at: chrono::NaiveDate,
}

pub async fn consumptions_create(
    state: State<AppState>,
    session: Session,
    Form(form): Form<CreateConsumptionForm>,
) -> Result<Response, AppError> {
    let mut food_id = None::<String>;
    let mut recipe_id = None::<String>;

    if form.consumable_type == "food" {
        let food = state.db.get_food(&form.consumable_id).await?;

        if !authz(&session, &food.user_id) {
            return Err(AppError::Forbidden);
        }

        food_id = Some(food.id.clone());
    } else if form.consumable_type == "recipe" {
        let recipe = state.db.get_recipe(&form.consumable_id).await?;

        if !authz(&session, &recipe.user_id) {
            return Err(AppError::Forbidden);
        }

        recipe_id = Some(recipe.id.clone());
    } else {
        return Err(AppError::InvalidConsumableType);
    }

    state
        .db
        .add_consumption(
            &session.user_id,
            food_id.as_deref(),
            recipe_id.as_deref(),
            form.quantity,
            &form.consumed_at,
        )
        .await?;

    Ok(redirect_to(AppUrl::DaySummary(form.consumed_at)))
}

pub async fn consumptions_read(
    state: State<AppState>,
    session: Session,
    Path(id): Path<String>,
) -> Result<Response, AppError> {
    let consumption = state.db.get_consumption(&id).await?;
    let consumables = state.db.get_consumables(&session.user_id).await?;

    if !authz(&session, &consumption.user_id) {
        return Err(AppError::Forbidden);
    }

    Ok(render_html(html::consumptions_update_page(
        consumption,
        consumables,
    )))
}

#[derive(Debug, serde::Deserialize)]
pub struct UpdateConsumptionForm {
    consumable_id: String,
    consumable_type: String,
    quantity: f64,
    consumed_at: chrono::NaiveDate,
}

pub async fn consumptions_update(
    state: State<AppState>,
    session: Session,
    Path(id): Path<String>,
    Form(form): Form<UpdateConsumptionForm>,
) -> Result<Response, AppError> {
    let consumption = state.db.get_consumption(&id).await?;

    if !authz(&session, &consumption.user_id) {
        return Err(AppError::Forbidden);
    }

    let mut food_id = None::<String>;
    let mut recipe_id = None::<String>;

    if form.consumable_type == "food" {
        let food = state.db.get_food(&form.consumable_id).await?;

        if !authz(&session, &food.user_id) {
            return Err(AppError::Forbidden);
        }

        food_id = Some(food.id.clone());
    } else if form.consumable_type == "recipe" {
        let recipe = state.db.get_recipe(&form.consumable_id).await?;

        if !authz(&session, &recipe.user_id) {
            return Err(AppError::Forbidden);
        }

        recipe_id = Some(recipe.id.clone());
    } else {
        return Err(AppError::InvalidConsumableType);
    }

    state
        .db
        .update_consumption(
            &id,
            food_id.as_deref(),
            recipe_id.as_deref(),
            form.quantity,
            &form.consumed_at,
        )
        .await?;

    Ok(redirect_to(AppUrl::DaySummary(consumption.consumed_at)))
}

pub async fn consumptions_delete(
    state: State<AppState>,
    session: Session,
    Path(id): Path<String>,
) -> Result<Response, AppError> {
    let consumption = state.db.get_consumption(&id).await?;

    if !authz(&session, &consumption.user_id) {
        return Err(AppError::Forbidden);
    }

    state.db.delete_consumption(&id).await?;

    Ok(redirect_to(AppUrl::DaySummary(consumption.consumed_at)))
}

pub async fn recipes_index(state: State<AppState>, session: Session) -> Result<Response, AppError> {
    let recipes = state.db.get_recipes(&session.user_id).await?;

    Ok(render_html(html::recipes_page(recipes)))
}

#[derive(Debug, serde::Deserialize)]
pub struct CreateRecipeForm {
    name: String,
    quantity: f64,
    hidden: Option<bool>,
    starred: Option<bool>,
}

pub async fn recipes_create(
    state: State<AppState>,
    session: Session,
    Form(form): Form<CreateRecipeForm>,
) -> Result<Response, AppError> {
    state
        .db
        .add_recipe(
            &session.user_id,
            &form.name,
            form.quantity,
            form.hidden.is_some(),
            form.starred.is_some(),
        )
        .await?;

    Ok(redirect_to(AppUrl::Recipes))
}

pub async fn recipes_read(
    state: State<AppState>,
    session: Session,
    Path(id): Path<String>,
) -> Result<Response, AppError> {
    let recipe = state.db.get_recipe(&id).await?;

    if !authz(&session, &recipe.user_id) {
        return Err(AppError::Forbidden);
    }

    let consumptions = state
        .db
        .get_consumptions(&session.user_id, ConsumptionFilter::RecipeId(id.clone()))
        .await?;
    let ingredients = state.db.get_ingredients(&id).await?;
    let consumables = state.db.get_consumables(&session.user_id).await?;

    Ok(render_html(html::recipes_update_page(
        recipe,
        consumptions,
        ingredients,
        consumables,
    )))
}

#[derive(Debug, serde::Deserialize)]
pub struct UpdateRecipeForm {
    name: String,
    quantity: f64,
    hidden: Option<bool>,
    starred: Option<bool>,
}

pub async fn recipes_update(
    state: State<AppState>,
    session: Session,
    Path(id): Path<String>,
    Form(form): Form<UpdateRecipeForm>,
) -> Result<Response, AppError> {
    let recipe = state.db.get_recipe(&id).await?;

    if !authz(&session, &recipe.user_id) {
        return Err(AppError::Forbidden);
    }

    state
        .db
        .update_recipe(
            &id,
            &form.name,
            form.quantity,
            form.hidden.is_some(),
            form.starred.is_some(),
        )
        .await?;

    Ok(redirect_to(AppUrl::RecipesId(id)))
}

pub async fn recipes_delete(
    state: State<AppState>,
    session: Session,
    Path(id): Path<String>,
) -> Result<Response, AppError> {
    let recipe = state.db.get_recipe(&id).await?;

    if !authz(&session, &recipe.user_id) {
        return Err(AppError::Forbidden);
    }

    state.db.delete_recipe(&id).await?;

    Ok(redirect_to(AppUrl::Recipes))
}

#[derive(Debug, serde::Deserialize)]
pub struct CreateIngredientForm {
    recipe_id: String,
    consumable_id: String,
    quantity: f64,
}

pub async fn ingredients_create(
    state: State<AppState>,
    session: Session,
    Form(form): Form<CreateIngredientForm>,
) -> Result<Response, AppError> {
    let recipe = state.db.get_recipe(&form.recipe_id).await?;

    if !authz(&session, &recipe.user_id) {
        return Err(AppError::Forbidden);
    }

    let food = state.db.get_food(&form.consumable_id).await?;

    if !authz(&session, &food.user_id) {
        return Err(AppError::Forbidden);
    }

    state
        .db
        .add_ingredient(
            &session.user_id,
            &form.recipe_id,
            &form.consumable_id,
            form.quantity,
        )
        .await?;
    Ok(redirect_to(AppUrl::RecipesId(form.recipe_id)))
}

pub async fn ingredients_read(
    state: State<AppState>,
    session: Session,
    Path(id): Path<String>,
) -> Result<Response, AppError> {
    let ingredient = state.db.get_ingredient(&id).await?;
    let consumables = state.db.get_consumables(&session.user_id).await?;

    if !authz(&session, &ingredient.user_id) {
        return Err(AppError::Forbidden);
    }

    Ok(render_html(html::ingredients_update_page(
        &ingredient,
        consumables,
    )))
}

#[derive(Debug, serde::Deserialize)]
pub struct UpdateIngredientForm {
    consumable_id: String,
    quantity: f64,
}

pub async fn ingredients_update(
    state: State<AppState>,
    session: Session,
    Path(id): Path<String>,
    Form(form): Form<UpdateIngredientForm>,
) -> Result<Response, AppError> {
    let ingredient = state.db.get_ingredient(&id).await?;

    if !authz(&session, &ingredient.user_id) {
        return Err(AppError::Forbidden);
    }

    let food = state.db.get_food(&form.consumable_id).await?;

    if !authz(&session, &food.user_id) {
        return Err(AppError::Forbidden);
    }

    state
        .db
        .update_ingredient(&id, &form.consumable_id, form.quantity)
        .await?;

    Ok(redirect_to(AppUrl::RecipesId(ingredient.recipe_id)))
}

pub async fn ingredients_delete(
    state: State<AppState>,
    session: Session,
    Path(id): Path<String>,
) -> Result<Response, AppError> {
    let ingredient = state.db.get_ingredient(&id).await?;

    if !authz(&session, &ingredient.user_id) {
        return Err(AppError::Forbidden);
    }

    state.db.delete_ingredient(&id).await?;

    Ok(redirect_to(AppUrl::RecipesId(ingredient.recipe_id)))
}

pub async fn account_read(state: State<AppState>, session: Session) -> Result<Response, AppError> {
    let mut entries = HashMap::new();

    let weights = state.db.get_weights(&session.user_id).await?;
    for weight in weights {
        entries
            .entry(weight.measured_at)
            .or_insert(html::AccountSummaryRow {
                date: weight.measured_at,
                weight: Some((weight.id, weight.weight)),
                kcal: None,
                protein: None,
            });
    }

    let consumptions = state
        .db
        .get_consumptions(&session.user_id, ConsumptionFilter::None)
        .await?;
    for consumption in consumptions {
        entries
            .entry(consumption.consumed_at)
            .and_modify(|r| {
                r.kcal = Some(r.kcal.unwrap_or(0.0) + consumption.kcal);
                r.protein = Some(r.protein.unwrap_or(0.0) + consumption.protein);
            })
            .or_insert(html::AccountSummaryRow {
                date: consumption.consumed_at,
                weight: None,
                kcal: Some(consumption.kcal),
                protein: Some(consumption.protein),
            });
    }

    let mut rows = entries
        .values()
        .cloned()
        .collect::<Vec<AccountSummaryRow>>();

    rows.sort_by(|a, b| b.date.cmp(&a.date));

    Ok(render_html(html::account_page(rows)))
}

pub async fn account_login_form(session: Option<Session>) -> Result<Response, AppError> {
    if session.is_some() {
        return Ok(redirect_to(AppUrl::Home));
    }

    Ok(render_html(html::login_page(None, None)))
}

#[derive(Debug, serde::Deserialize)]
pub struct LoginForm {
    username: String,
    password: String,
}

pub async fn account_login(
    state: State<AppState>,
    session: Option<Session>,
    Form(form): Form<LoginForm>,
) -> Result<Response, AppError> {
    if session.is_some() {
        return Ok(redirect_to(AppUrl::Home));
    }

    let error_response = Ok(render_html(html::login_page(
        Some(form.username.clone()),
        Some("Invalid username or password.".to_string()),
    )));

    match state.db.get_user(&form.username).await? {
        None => error_response,
        Some(user) => match verify_password(&form.password, &user.password_hash) {
            Err(_) => error_response,
            Ok(_) => {
                let session_id = state.db.create_session(&user.id).await?;
                Ok(redirect_with_session_cookie_response(
                    AppUrl::Home,
                    Some(session_id),
                ))
            }
        },
    }
}

pub async fn account_logout(
    state: State<AppState>,
    session: Session,
) -> Result<Response, AppError> {
    state.db.delete_session(&session.session_id).await?;
    Ok(redirect_with_session_cookie_response(AppUrl::Home, None))
}

pub async fn account_export(
    state: State<AppState>,
    session: Session,
) -> Result<Response, AppError> {
    let filename = format!("{}-matrafl.json", chrono::Utc::now().date_naive());
    let data = state.db.export_data(&session.user_id).await?;
    Ok(Response::builder()
        .header("Content-Type", "application/json")
        .header(
            "Content-Disposition",
            format!("attachment; filename={}", filename).as_str(),
        )
        .body(Body::from(serde_json::to_string(&data).unwrap()))?)
}

fn redirect_with_session_cookie_response(url: AppUrl, session_id: Option<String>) -> Response {
    let mut cookie = Cookie::new("MATRAFL_SESSION", session_id.clone().unwrap_or_default());
    cookie.set_expires(match session_id {
        Some(_) => OffsetDateTime::now_utc() + Duration::days(SESSION_DAYS),
        None => OffsetDateTime::now_utc() - Duration::days(1),
    });
    cookie.set_http_only(true);
    // With SameSite=Strict, the cookie will not be sent when opening the PWA.
    cookie.set_same_site(SameSite::Lax);
    cookie.set_path("/");

    (
        StatusCode::SEE_OTHER,
        [
            (
                header::LOCATION,
                HeaderValue::try_from(url.to_string().as_str()).unwrap(),
            ),
            (
                header::SET_COOKIE,
                HeaderValue::try_from(cookie.encoded().to_string().as_str()).unwrap(),
            ),
        ],
    )
        .into_response()
}

fn verify_password(password: &str, hash: &str) -> Result<(), AppError> {
    let parsed_hash = PasswordHash::new(hash)?;
    Ok(Argon2::default().verify_password(password.as_bytes(), &parsed_hash)?)
}

fn render_html(markup: Markup) -> Response {
    Html(markup.into_string()).into_response()
}
