use std::env;

use maud::{html, Markup, PreEscaped, Render, DOCTYPE};

use crate::{db, AppUrl};

#[derive(Debug, PartialEq, Eq)]
pub enum NavItem {
    None,
    Home,
    Weights,
    Foods,
    Recipes,
    Account,
}

pub fn day_summary_page(
    date: chrono::NaiveDate,
    weight: Option<db::WeightWithDate>,
    consumptions: Vec<db::Consumption>,
    consumables: Vec<db::Consumable>,
) -> Markup {
    let prev_day = date.pred_opt().unwrap();
    let next_day = date.succ_opt().unwrap();

    let mut total_kcal = 0.0;
    let mut total_fat = 0.0;
    let mut total_carbs = 0.0;
    let mut total_protein = 0.0;

    for row in &consumptions {
        total_kcal += row.kcal;
        total_fat += row.fat;
        total_carbs += row.carbs;
        total_protein += row.protein;
    }

    page_with_layout(
        &NavItem::Home,
        date.to_string().as_str(),
        html! {
            div class="home-header" {
                a href=(AppUrl::DaySummary(prev_day)) class="button gray" { (PhosphorIcon::CaretLeft) }
                div {
                    span {(date)}
                    @if let Some(weight) = &weight {
                        a href=(AppUrl::WeightsId(weight.id.clone())) { (weight.weight) " kg" }
                    } @else {
                        span { "\u{00a0}" }
                    }
                }
                a href=(AppUrl::DaySummary(next_day)) class="button gray" { (PhosphorIcon::CaretRight) }
            }
            @if weight.is_none() {
                form method="post" action=(AppUrl::Weights) {
                    (input_group_number("weight", "Weight", "weight", ""))
                    input type="hidden" name="measured_at" value=(date);
                    input type="hidden" name="redirect_to" value="DaySummary";
                    button type="submit" { "Save" };
                }
            }
            div class="home-summary" {
                div class="summary" {
                    div { span { (format!("{:.0}", total_kcal)) } span { "kcal" } }
                    div { span { (format!("{:.0}", total_fat)) } span { "fat" } }
                    div { span { (format!("{:.0}", total_carbs)) } span { "carbs" } }
                    div { span { (format!("{:.0}", total_protein)) } span { "protein" } }
                }
                button type="button" class="green" data-toggler data-toggler-target="form.consumption" { (PhosphorIcon::Plus) };
            }
            form method="post" action=(AppUrl::Consumptions) class="consumption" hidden[true] {
                (input_group_date("input-date", "Date", "consumed_at", &date.to_string()));
                (food_select_trigger(None));
                (input_group_number("input-quantity", "Quantity", "quantity", ""));
                button type="submit" { "Save" };
            }
            (food_select_dialog(consumables))
            div class="macro-cards" {
                @for consumption in consumptions {
                    (consumption_card(&consumption, false))
                }
            }
        },
    )
}

pub fn weights_page(weights: Vec<db::WeightWithDate>) -> Markup {
    page_with_layout(
        &NavItem::Weights,
        "Weights",
        html! {
            form method="post" action=(AppUrl::Weights) {
                (input_group_date("measured_at", "Date", "measured_at", &chrono::Utc::now().date_naive().to_string()));
                (input_group_number("weight", "Weight", "weight", ""))
                button type="submit" { "Save" };
            }
            div class="table-container" {
                table {
                    thead {
                        tr {
                            th { "Date" }
                            th { "Weight" }
                        }
                    }
                    tbody {
                        @if weights.is_empty() {
                            tr {
                                td colspan="2" class="text-center" { "No data." }
                            }
                        }
                        @for row in weights {
                            tr {
                                td { a href=(AppUrl::DaySummary(row.measured_at)) { (row.measured_at) } }
                                td { a href=(AppUrl::WeightsId(row.id.clone())) { (row.weight) } }
                            }
                        }
                    }
                }
            }
        },
    )
}

pub fn weights_update_page(weight: &db::WeightWithDate) -> Markup {
    page_with_layout(
        &NavItem::Weights,
        "Update Weight",
        html! {
            form method="post" action=(AppUrl::WeightsId(weight.id.clone())) {
                (input_group_number("weight", "Weight", "weight", &weight.weight.to_string()))
                (input_group_date("measured_at", "Date", "measured_at", &weight.measured_at.to_string()));
                button type="submit" { "Save" };
            }
            form method="post" action=(AppUrl::WeightsIdDelete(weight.id.clone())) data-confirm-delete {
                button type="submit" class="red" { "Delete" };
            }
        },
    )
}

pub fn foods_page(foods: Vec<db::Food>) -> Markup {
    page_with_layout(
        &NavItem::Foods,
        "Foods",
        html! {
            div class="search-container" {
                label class="search-wrapper" {
                    (PhosphorIcon::MagnifyingGlass)
                    input type="text" autofocus data-search-trigger;
                }
                button type="button" class="green" data-toggler data-toggler-target="form.food" { (PhosphorIcon::Plus) };
            }
            form method="post" action=(AppUrl::Foods) class="food" hidden[true] autocomplete="off" {
                (input_group_text("input-name", "Name", "name", ""));
                (input_group_number("input-kcal", "Calories", "kcal", ""));
                (input_group_number("input-fat", "Fat", "fat", ""));
                (input_group_number("input-carbs", "Carbs", "carbs", ""));
                (input_group_number("input-protein", "Protein", "protein", ""));
                (input_group_checkbox("input-hidden", "Hidden", "hidden", false));
                (input_group_checkbox("input-starred", "Starred", "starred", false));
                button type="submit" { "Save" };
            }
            div class="macro-cards" {
                @for food in foods {
                    (macro_card(
                        food.name.as_str(),
                        AppUrl::FoodsId(food.id.clone()),
                        None,
                        None,
                        db::Macros {
                            kcal: food.kcal,
                            fat: food.fat,
                            carbs: food.carbs,
                            protein: food.protein
                        }
                    ))
                }
            }
        },
    )
}

pub fn foods_update_page(food: db::Food, consumptions: Vec<db::Consumption>) -> Markup {
    page_with_layout(
        &NavItem::Foods,
        "Update Food",
        html! {
            form method="post" action=(AppUrl::FoodsId(food.id.clone())) class="food" {
                (input_group_text("input-name", "Name", "name", &food.name));
                (input_group_number("input-kcal", "Calories", "kcal", &food.kcal.to_string()));
                (input_group_number("input-fat", "Fat", "fat", &food.fat.to_string()));
                (input_group_number("input-carbs", "Carbs", "carbs", &food.carbs.to_string()));
                (input_group_number("input-protein", "Protein", "protein", &food.protein.to_string()));
                (input_group_checkbox("input-hidden", "Hidden", "hidden", food.hidden_at.is_some()));
                (input_group_checkbox("input-starred", "Starred", "starred", food.starred_at.is_some()));
                button type="submit" { "Save" };
            }
            form method="post" action=(AppUrl::FoodsIdDelete(food.id.clone())) data-confirm-delete {
                button type="submit" class="red" { "Delete" };
            }
            h2 { "Consumptions" }
            div class="macro-cards" {
                @for consumption in consumptions {
                    (consumption_card(&consumption, true))
                }
            }
        },
    )
}

pub fn consumptions_update_page(
    consumption: db::Consumption,
    consumables: Vec<db::Consumable>,
) -> Markup {
    page_with_layout(
        &NavItem::Home,
        "Update Consumption",
        html! {
            form method="post" action=(AppUrl::ConsumptionsId(consumption.id.clone())) {
                (input_group_date("input-date", "Date", "consumed_at", &consumption.consumed_at.to_string()));
                (food_select_trigger(consumables.iter().find(|c| c.id == consumption.consumable_id)));
                (input_group_number("input-quantity", "Quantity", "quantity", &consumption.quantity.to_string()));
                button type="submit" { "Save" };
            }
            (food_select_dialog(consumables))
            form method="post" action=(AppUrl::ConsumptionsIdDelete(consumption.id.clone())) data-confirm-delete {
                button type="submit" class="red" { "Delete" };
            }
        },
    )
}

pub fn recipes_page(recipes: Vec<db::Recipe>) -> Markup {
    page_with_layout(
        &NavItem::Recipes,
        "Recipes",
        html! {
            div class="search-container" {
                label class="search-wrapper" {
                    (PhosphorIcon::MagnifyingGlass)
                    input type="text" autofocus data-search-trigger;
                }
                button type="button" class="green" data-toggler data-toggler-target="form.recipe" { (PhosphorIcon::Plus) };
            }
            form method="post" action=(AppUrl::Recipes) class="recipe" hidden[true] {
                (input_group_text("input-name", "Name", "name", ""));
                (input_group_number("input-quantity", "Quantity", "quantity", "1"));
                button type="submit" { "Save" };
            }
            div class="macro-cards" {
                @for row in recipes {
                    (macro_card(
                        row.name.as_str(),
                        AppUrl::RecipesId(row.id.clone()),
                        Some(row.quantity),
                        None,
                        db::Macros {
                            kcal: row.kcal,
                            fat: row.fat,
                            carbs: row.carbs,
                            protein: row.protein
                        }
                    ))
                }
            }
        },
    )
}

pub fn recipes_update_page(
    recipe: db::Recipe,
    consumptions: Vec<db::Consumption>,
    ingredients: Vec<db::Ingredient>,
    consumables: Vec<db::Consumable>,
) -> Markup {
    let mut total_kcal = 0.0;
    let mut total_fat = 0.0;
    let mut total_carbs = 0.0;
    let mut total_protein = 0.0;

    for row in &ingredients {
        total_kcal += row.kcal;
        total_fat += row.fat;
        total_carbs += row.carbs;
        total_protein += row.protein;
    }

    page_with_layout(
        &NavItem::Recipes,
        "Update Recipe",
        html! {
            form method="post" action=(AppUrl::RecipesId(recipe.id.clone())) class="recipe" {
                (input_group_text("input-name", "Name", "name", &recipe.name));
                (input_group_number("input-kcal", "Quantity", "quantity", &recipe.quantity.to_string()));
                (input_group_checkbox("input-hidden", "Hidden", "hidden", recipe.hidden_at.is_some()));
                (input_group_checkbox("input-starred", "Starred", "starred", recipe.starred_at.is_some()));
                button type="submit" { "Save" };
            }
            div class="home-summary" {
                div class="summary" {
                    div { span { (format!("{:.0}", total_kcal)) } span { "kcal" } }
                    div { span { (format!("{:.0}", total_fat)) } span { "fat" } }
                    div { span { (format!("{:.0}", total_carbs)) } span { "carbs" } }
                    div { span { (format!("{:.0}", total_protein)) } span { "protein" } }
                }
                button type="button" class="green" data-toggler data-toggler-target="form.ingredient" { (PhosphorIcon::Plus) };
            }
            form method="post" action=(AppUrl::Ingredients) class="ingredient" hidden[true] {
                input type="hidden" name="recipe_id" value=(recipe.id);
                (food_select_trigger(None));
                (input_group_number("input-quantity", "Quantity", "quantity", ""));
                button type="submit" { "Save" };
            }
            h2 { "Ingredients" }
            div class="macro-cards" {
                @for ingredient in ingredients {
                    (ingredient_card(&ingredient))
                }
            }
            h2 { "Consumptions" }
            div class="macro-cards" {
                @for consumption in consumptions {
                    (consumption_card(&consumption, true))
                }
            }
            form method="post" action=(AppUrl::RecipesIdDelete(recipe.id.clone())) data-confirm-delete {
                button type="submit" class="red" { "Delete" };
            }
            (food_select_dialog(consumables))
        },
    )
}

pub fn ingredients_update_page(
    ingredient: &db::Ingredient,
    consumables: Vec<db::Consumable>,
) -> Markup {
    page_with_layout(
        &NavItem::Recipes,
        "Update Ingredient",
        html! {
            form method="post" action=(AppUrl::IngredientsId(ingredient.id.clone())) class="ingredient" {
                (food_select_trigger(consumables.iter().find(|c| c.id == ingredient.food_id)));
                (input_group_number("input-kcal", "Quantity", "quantity", &ingredient.quantity.to_string()));
                button type="submit" { "Save" };
            }
            form method="post" action=(AppUrl::IngredientsIdDelete(ingredient.id.clone())) data-confirm-delete {
                button type="submit" class="red" { "Delete" };
            }
            (food_select_dialog(consumables))
        },
    )
}

#[derive(Clone)]
pub struct AccountSummaryRow {
    pub date: chrono::NaiveDate,
    pub weight: Option<(String, f64)>,
    pub kcal: Option<f64>,
    pub protein: Option<f64>,
}

pub fn account_page(rows: Vec<AccountSummaryRow>) -> Markup {
    page_with_layout(
        &NavItem::Account,
        "Account",
        html! {
            div.grid-col-2 {
                form method="post" action=(AppUrl::AccountExport) {
                    button type="submit" class="gray" { "Export"};
                }
                form method="post" action=(AppUrl::AccountLogout) {
                    button type="submit" class="gray" { "Logout"};
                }
            }
            div.table-container {
                table {
                    thead {
                        tr {
                            th { "Date" }
                            th { "Weight" }
                            th { "kcal" }
                            th { "Protein" }
                        }
                    }
                    tbody {
                        @if rows.is_empty() {
                            tr {
                                td colspan="4" class="text-center" { "No data." }
                            }
                        }
                        @for row in rows {
                            tr {
                                td { a href=(AppUrl::DaySummary(row.date)) { (row.date) } }
                                td { @if let Some((weight_id, weight_value)) = row.weight {
                                    a href=(AppUrl::WeightsId(weight_id)) { (weight_value) }
                                } }
                                td { (row.kcal.map(fmt_macro).unwrap_or_default()) }
                                td { (row.protein.map(fmt_macro).unwrap_or_default()) }
                            }
                        }
                    }
                }
            }
            div.build-info {
                "Build " (env!("VERGEN_BUILD_TIMESTAMP"))
            }
        },
    )
}

pub fn login_page(username: Option<String>, error_msg: Option<String>) -> Markup {
    html!(
        (DOCTYPE)
        html {
            (html_head("Login"))
            body {
                form method="post" action=(AppUrl::AccountLogin) class="login" {
                    h1 { "Login" }
                    @if let Some(error_msg) = error_msg {
                        div class="error" { (error_msg) }
                    }
                    (input_group_text("input-username", "Username", "username", &username.unwrap_or_default()))
                    (input_group_password("input-password", "Password", "password"))
                    button type="submit" { "Login" };
                }
            }
        }
    )
}

pub fn error_page() -> Markup {
    page_with_layout(
        &NavItem::None,
        "Error",
        html! {
            div class="error" {
                "An error occurred."
            }
        },
    )
}

fn page_with_layout(active_nav_item: &NavItem, page_title: &str, main_content: Markup) -> Markup {
    html!(
        (DOCTYPE)
        html {
            (html_head(page_title))
            body {
                div class="container" {
                    nav {
                        ul {
                            li.active[active_nav_item.eq(&NavItem::Home)] { a href=(AppUrl::Home) { (PhosphorIcon::House) } }
                            li.active[active_nav_item.eq(&NavItem::Weights)] { a href=(AppUrl::Weights) { (PhosphorIcon::Scales) } }
                            li.active[active_nav_item.eq(&NavItem::Foods)] { a href=(AppUrl::Foods) { (PhosphorIcon::Orange) } }
                            li.active[active_nav_item.eq(&NavItem::Recipes)] { a href=(AppUrl::Recipes) { (PhosphorIcon::CookingPot) } }
                            li.active[active_nav_item.eq(&NavItem::Account)] { a href=(AppUrl::Account) { (PhosphorIcon::User) } }
                        }
                    }
                    (main_content)
                }
            }
        }
    )
}

fn input_group_date(id: &str, label: &str, name: &str, value: &str) -> Markup {
    html! {
        div.input-group {
            label for=(id) { (label) }
            input type="date" id=(id) name=(name) value=(value) required;
        }
    }
}

fn input_group_text(id: &str, label: &str, name: &str, value: &str) -> Markup {
    html! {
        div.input-group {
            label for=(id) { (label) }
            input type="text" id=(id) name=(name) value=(value) required autocomplete="new-text";
        }
    }
}

fn input_group_password(id: &str, label: &str, name: &str) -> Markup {
    html! {
        div.input-group {
            label for=(id) { (label) }
            input type="password" id=(id) name=(name) required;
        }
    }
}

fn input_group_number(id: &str, label: &str, name: &str, value: &str) -> Markup {
    html! {
        div.input-group {
            label for=(id) { (label) }
            input type="number" id=(id) name=(name) value=(value) min="0.01" step="0.01" autocomplete="new-number" required;
        }
    }
}

fn input_group_checkbox(id: &str, label: &str, name: &str, checked: bool) -> Markup {
    html! {
        div.input-group {
            label for=(id) { (label) }
            input type="checkbox" value="true" id=(id) name=(name) checked[checked];
        }
    }
}

fn fmt_macro(macro_value: f64) -> String {
    if macro_value.fract() == 0.0 {
        format!("{:.0}", macro_value)
    } else {
        format!("{:.1}", macro_value)
    }
}

enum PhosphorIcon {
    Plus,
    CaretLeft,
    CaretRight,
    House,
    Scales,
    Orange,
    CookingPot,
    User,
    MagnifyingGlass,
    X,
    Star,
    ArrowSquareUpRight,
}

// Icons by https://phosphoricons.com/
// License: MIT
impl Render for PhosphorIcon {
    fn render(&self) -> Markup {
        PreEscaped(match self {
            PhosphorIcon::Plus => r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 256 256"><rect width="256" height="256" fill="none"/><line x1="40" y1="128" x2="216" y2="128" fill="none" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="16"/><line x1="128" y1="40" x2="128" y2="216" fill="none" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="16"/></svg>"#,
            PhosphorIcon::CaretLeft => r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 256 256"><rect width="256" height="256" fill="none"/><polyline points="160 208 80 128 160 48" fill="none" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="16"/></svg>"#,
            PhosphorIcon::CaretRight => r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 256 256"><rect width="256" height="256" fill="none"/><polyline points="96 48 176 128 96 208" fill="none" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="16"/></svg>"#,
            PhosphorIcon::House => r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 256 256"><rect width="256" height="256" fill="none"/><path d="M104,216V152h48v64h64V120a8,8,0,0,0-2.34-5.66l-80-80a8,8,0,0,0-11.32,0l-80,80A8,8,0,0,0,40,120v96Z" fill="none" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="16"/></svg>"#,
            PhosphorIcon::Scales => r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 256 256"><rect width="256" height="256" fill="none"/><line x1="128" y1="40" x2="128" y2="216" fill="none" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="16"/><line x1="104" y1="216" x2="152" y2="216" fill="none" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="16"/><line x1="56" y1="88" x2="200" y2="56" fill="none" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="16"/><path d="M24,168c0,17.67,20,24,32,24s32-6.33,32-24L56,88Z" fill="none" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="16"/><path d="M168,136c0,17.67,20,24,32,24s32-6.33,32-24L200,56Z" fill="none" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="16"/></svg>"#,
            PhosphorIcon::Orange => r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 256 256"><rect width="256" height="256" fill="none"/><circle cx="128" cy="152" r="80" fill="none" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="16"/><path d="M128,72h0a56,56,0,0,1,56-56h8a56,56,0,0,1-56,56Z" fill="none" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="16"/><path d="M128,72h0A56,56,0,0,0,72,16H64" fill="none" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="16"/><path d="M176,160a49.52,49.52,0,0,1-40,40" fill="none" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="16"/></svg>"#,
            PhosphorIcon::CookingPot => r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 256 256"><rect width="256" height="256" fill="none"/><line x1="96" y1="16" x2="96" y2="48" fill="none" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="16"/><line x1="128" y1="16" x2="128" y2="48" fill="none" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="16"/><line x1="160" y1="16" x2="160" y2="48" fill="none" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="16"/><path d="M40,80H216V184a24,24,0,0,1-24,24H64a24,24,0,0,1-24-24Z" fill="none" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="16"/><line x1="248" y1="96" x2="216" y2="120" fill="none" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="16"/><line x1="8" y1="96" x2="40" y2="120" fill="none" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="16"/></svg>"#,
            PhosphorIcon::User => r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 256 256"><rect width="256" height="256" fill="none"/><circle cx="128" cy="96" r="64" fill="none" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="16"/><path d="M32,216c19.37-33.47,54.55-56,96-56s76.63,22.53,96,56" fill="none" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="16"/></svg>"#,
            PhosphorIcon::MagnifyingGlass => r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 256 256"><rect width="256" height="256" fill="none"/><circle cx="112" cy="112" r="80" fill="none" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="16"/><line x1="168.57" y1="168.57" x2="224" y2="224" fill="none" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="16"/></svg>"#,
            PhosphorIcon::X => r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 256 256"><rect width="256" height="256" fill="none"/><line x1="200" y1="56" x2="56" y2="200" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="16"/><line x1="200" y1="200" x2="56" y2="56" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="16"/></svg>"#,
            PhosphorIcon::Star => r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 256 256"><rect width="256" height="256" fill="none"/><path d="M128,189.09l54.72,33.65a8.4,8.4,0,0,0,12.52-9.17l-14.88-62.79,48.7-42A8.46,8.46,0,0,0,224.27,94L160.36,88.8,135.74,29.2a8.36,8.36,0,0,0-15.48,0L95.64,88.8,31.73,94a8.46,8.46,0,0,0-4.79,14.83l48.7,42L60.76,213.57a8.4,8.4,0,0,0,12.52,9.17Z" fill="none" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="16"/></svg>"#,
            PhosphorIcon::ArrowSquareUpRight => r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 256 256"><rect width="256" height="256" fill="none"/><rect x="40" y="40" width="176" height="176" rx="8" transform="translate(0 256) rotate(-90)" fill="none" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="16"/><line x1="160" y1="96" x2="96" y2="160" fill="none" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="16"/><polyline points="112 96 160 96 160 144" fill="none" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="16"/></svg>"#
        }.to_string())
    }
}

fn food_select_trigger(consumable: Option<&db::Consumable>) -> Markup {
    html!(
        div.input-group.consumable-select-group {
            label for="input-food" { "Food" }
            button type="button" class="consumable-select-trigger" data-consumable-select-trigger {
                @if let Some(consumable) = consumable {
                    (consumable.name)
                } @else {
                    (PhosphorIcon::MagnifyingGlass)
                }
            }
            a href=(consumable.map(consumable_url).unwrap_or_default()) class="consumable-select-open" data-consumable-select-open { (PhosphorIcon::ArrowSquareUpRight) }
            input type="hidden" id="input-consumable-id" name="consumable_id" required data-consumable-select-id-input value=(consumable.map(|c| c.id.clone()).unwrap_or_default());
            input type="hidden" id="input-consumable-type" name="consumable_type" required data-consumable-select-type-input value=(consumable.map(|c| c.ctype.as_str()).unwrap_or_default());
        }
    )
}

fn consumable_url(consumable: &db::Consumable) -> PreEscaped<String> {
    match consumable.ctype {
        db::ConsumableType::Food => AppUrl::FoodsId(consumable.id.clone()).render(),
        db::ConsumableType::Recipe => AppUrl::RecipesId(consumable.id.clone()).render(),
    }
}

fn food_select_dialog(consumables: Vec<db::Consumable>) -> Markup {
    html!(
        dialog class="consumable-select-dialog" data-consumable-select-dialog {
            div.wrapper {
                div.header {
                    input type="text" autofocus;
                    button type="button" class="gray" data-consumable-select-closer { (PhosphorIcon::X) }
                }
                div.options {
                    @for consumable in consumables {
                        button.option type="button" data-consumable-id=(consumable.id) data-consumable-type=(consumable.ctype.as_str()) data-consumable-name=(consumable.name) data-consumable-url=(consumable_url(&consumable)) {
                            span class="name" {
                                (consumable.name)
                            }
                            @if consumable.is_starred {
                                span class="starred" {
                                    (PhosphorIcon::Star)
                                }
                            }
                            @if consumable.ctype == db::ConsumableType::Recipe {
                                span class="recipe" {
                                    (PhosphorIcon::CookingPot)
                                }
                            }
                        }
                    }
                }
            }
        }
    )
}

fn macro_card(
    name: &str,
    url: AppUrl,
    quantity: Option<f64>,
    date: Option<chrono::NaiveDate>,
    macros: db::Macros,
) -> Markup {
    let classes = match (quantity, date) {
        (Some(_), Some(_)) => "macro-card with-quantity with-date",
        (Some(_), None) => "macro-card with-quantity",
        (None, Some(_)) => "macro-card with-date",
        (None, None) => "macro-card",
    };
    html! {
        div class=(classes) data-search-item=(name) {
            @if let Some(date) = date {
                a class="date" href=(AppUrl::DaySummary(date)) { (date) }
            }
            a class="name" href=(url) { (name) }
            @if let Some(quantity) = quantity {
                div class="quantity" { "×" (quantity) }
            }
            div class="kcal" { span { (fmt_macro(macros.kcal)) } span { "kcal" } }
            div class="fat" { span { (fmt_macro(macros.fat)) } span { "fat" } }
            div class="carbs" { span { (fmt_macro(macros.carbs)) } span { "carbs" } }
            div class="protein" { span { (fmt_macro(macros.protein)) } span { "protein" } }
        }
    }
}

fn consumption_card(consumption: &db::Consumption, date: bool) -> Markup {
    macro_card(
        consumption.consumable_name.as_str(),
        AppUrl::ConsumptionsId(consumption.id.clone()),
        Some(consumption.quantity),
        if date {
            Some(consumption.consumed_at)
        } else {
            None
        },
        db::Macros {
            kcal: consumption.kcal,
            fat: consumption.fat,
            carbs: consumption.carbs,
            protein: consumption.protein,
        },
    )
}

fn ingredient_card(ingredient: &db::Ingredient) -> Markup {
    macro_card(
        ingredient.food_name.as_str(),
        AppUrl::IngredientsId(ingredient.id.clone()),
        Some(ingredient.quantity),
        None,
        db::Macros {
            kcal: ingredient.kcal,
            fat: ingredient.fat,
            carbs: ingredient.carbs,
            protein: ingredient.protein,
        },
    )
}

fn html_head(page_title: &str) -> Markup {
    let build_timestamp = env!("VERGEN_BUILD_TIMESTAMP");
    html! {
        head {
            meta charset="utf-8";
            meta name="viewport" content="width=device-width, initial-scale=1";
            link rel="stylesheet" type="text/css" href=(format!("/assets/main.css?t={}", build_timestamp));
            link rel="icon" href=(format!("/assets/icon-32.png?t={}", build_timestamp));
            link rel="manifest" href=(format!("/assets/manifest.webmanifest?t={}", build_timestamp));
            script defer src=(format!("/assets/main.js?t={}", build_timestamp)) {}
            title { (page_title) " - Matrafl" }
        }
    }
}
