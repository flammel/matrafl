create table users (
    id text not null,
    username text not null,
    password_hash text not null,
    created_at datetime not null,
    updated_at datetime not null,
    primary key (id),
    unique (username)
);

create table weights (
    id text not null,
    user_id text not null,
    weight real not null,
    measured_at datetime not null,
    created_at datetime not null,
    updated_at datetime not null,
    primary key (id),
    foreign key (user_id) references users(id)
);

create table foods (
    id text not null,
    user_id text not null,
    name text not null,
    kcal real not null,
    fat real not null,
    carbs real not null,
    protein real not null,
    hidden_at datetime default null,
    starred_at datetime default null,
    created_at datetime not null,
    updated_at datetime not null,
    primary key (id),
    foreign key (user_id) references users(id)
);

create table consumptions (
    id text not null,
    user_id text not null,
    food_id text default null,
    recipe_id text default null,
    quantity real not null,
    consumed_at datetime not null,
    created_at datetime not null,
    updated_at datetime not null,
    primary key (id),
    foreign key (user_id) references users(id),
    foreign key (food_id) references foods(id),
    foreign key (recipe_id) references recipes(id)
);

create table recipes (
    id text not null,
    user_id text not null,
    name text not null,
    quantity real not null,
    hidden_at datetime default null,
    starred_at datetime default null,
    created_at datetime not null,
    updated_at datetime not null,
    primary key (id),
    foreign key (user_id) references users(id)
);

create table ingredients (
    id text not null,
    user_id text not null,
    recipe_id text not null,
    food_id text not null,
    quantity real not null,
    created_at datetime not null,
    updated_at datetime not null,
    primary key (id),
    foreign key (user_id) references users(id),
    foreign key (recipe_id) references recipes(id),
    foreign key (food_id) references foods(id)
);

create table sessions (
    id text not null,
    user_id text not null,
    created_at datetime not null,
    primary key (id),
    foreign key (user_id) references users(id)
);