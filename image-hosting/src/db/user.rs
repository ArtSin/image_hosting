#![cfg(feature = "ssr")]

use crate::user::User;

pub async fn get_user_by_id(id: i64) -> Result<Option<User>, sqlx::Error> {
    let db = crate::DB_CONN.get().unwrap();
    sqlx::query!(r#"select "name" from "users" where "id" = $1"#, id)
        .fetch_optional(db)
        .await
        .map(|x| x.map(|y| User { id, name: y.name }))
}

pub async fn get_user_id_by_name(name: &str) -> Result<Option<i64>, sqlx::Error> {
    let db = crate::DB_CONN.get().unwrap();
    sqlx::query!(r#"select "id" from "users" where "name" = $1"#, name)
        .fetch_optional(db)
        .await
        .map(|x| x.map(|y| y.id))
}

pub async fn get_user_with_password_hash_by_name(
    name: &str,
) -> Result<Option<(User, String)>, sqlx::Error> {
    let db = crate::DB_CONN.get().unwrap();
    sqlx::query!(
        r#"select "id", "password_hash" from "users" where "name" = $1"#,
        name
    )
    .fetch_optional(db)
    .await
    .map(|x| {
        x.map(|y| {
            (
                User {
                    id: y.id,
                    name: name.to_owned(),
                },
                y.password_hash,
            )
        })
    })
}

pub async fn insert_user(user: &mut User, password_hash: &str) -> Result<(), sqlx::Error> {
    let db = crate::DB_CONN.get().unwrap();
    user.id = sqlx::query!(
        r#"insert into "users" ("name", "password_hash") values ($1, $2) returning "id""#,
        user.name,
        password_hash
    )
    .fetch_one(db)
    .await?
    .id;
    Ok(())
}
