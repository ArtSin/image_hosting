#![cfg(feature = "ssr")]

use std::collections::HashMap;

use chrono::{DateTime, Utc};

use crate::{image::Image, image_votes::ImageVotes, user::User};

macro_rules! get_images_with_authors_and_votes {
    ($curr_user_id:ident, $where:literal, $order_by: literal, $( $var:expr ),*) => {
        sqlx::query!(
            r#"
            select
                i."id" as "image_id",
                i."format" as "format",
                i."title" as "title",
                i."author" as "author",
                i."timestamp" as "timestamp",
                u."name" as "author_name",
                (coalesce(sum(case when iv."upvote" is null then 0 else
                    (case when iv."upvote" then 1 else -1 end) end), 0)) as "rating!",
                iv_curr."upvote" as "curr_user_upvote?"
            from
                "images" i
            join
                "users" u on i."author" = u."id"
            left join
                "images_votes" iv on i."id" = iv."image_id"
            left join
                "images_votes" iv_curr on i."id" = iv_curr."image_id" and iv_curr."user_id" = $1
            "# + $where + r#"
            group by
                i."id", u."name", iv_curr."upvote"
            "# + $order_by,
            $curr_user_id
            $(
                ,$var
            )*
        )
    };
}

macro_rules! record_to_images_with_authors_and_votes {
    ($x: ident) => {
        (
            Image {
                id: $x.image_id,
                format: $x.format,
                title: $x.title,
                author: $x.author,
                timestamp: $x.timestamp,
            },
            User {
                id: $x.author,
                name: $x.author_name,
            },
            ImageVotes {
                image_id: $x.image_id,
                rating: $x.rating,
                curr_user_upvote: $x.curr_user_upvote,
            },
        )
    };
}

pub async fn get_all_images_with_authors_and_votes(
    curr_user_id: i64,
    count: i64,
    last_timestamp: Option<DateTime<Utc>>,
) -> Result<(Vec<(Image, User, ImageVotes)>, bool), sqlx::Error> {
    let db = crate::DB_CONN.get().unwrap();
    let last_timestamp = last_timestamp.unwrap_or(DateTime::<Utc>::MAX_UTC);
    get_images_with_authors_and_votes!(
        curr_user_id,
        r#"where i."timestamp" < $3"#,
        r#"order by i."timestamp" desc limit $2"#,
        count + 1,
        last_timestamp
    )
    .fetch_all(db)
    .await
    .map(|res| {
        let mut v: Vec<_> = res
            .into_iter()
            .map(|x| record_to_images_with_authors_and_votes!(x))
            .collect();
        let mut last_page = true;
        if v.len() == (count + 1) as usize {
            last_page = false;
            v.pop();
        }
        (v, last_page)
    })
}

pub async fn get_all_images_with_authors_and_votes_by_author(
    curr_user_id: i64,
    count: i64,
    author_id: i64,
    last_timestamp: Option<DateTime<Utc>>,
) -> Result<(Vec<(Image, User, ImageVotes)>, bool), sqlx::Error> {
    let db = crate::DB_CONN.get().unwrap();
    let last_timestamp = last_timestamp.unwrap_or(DateTime::<Utc>::MAX_UTC);
    get_images_with_authors_and_votes!(
        curr_user_id,
        r#"where i."author" = $3 and i."timestamp" < $4"#,
        r#"order by i."timestamp" desc limit $2"#,
        count + 1,
        author_id,
        last_timestamp
    )
    .fetch_all(db)
    .await
    .map(|res| {
        let mut v: Vec<_> = res
            .into_iter()
            .map(|x| record_to_images_with_authors_and_votes!(x))
            .collect();
        let mut last_page = true;
        if v.len() == (count + 1) as usize {
            last_page = false;
            v.pop();
        }
        (v, last_page)
    })
}

pub async fn get_images_with_authors_and_votes_by_ids(
    curr_user_id: i64,
    ids: Vec<i64>,
) -> Result<Vec<(Image, User, ImageVotes)>, sqlx::Error> {
    let db = crate::DB_CONN.get().unwrap();
    get_images_with_authors_and_votes!(
        curr_user_id,
        r#"where i."id" in (select unnest($2::bigint[]))"#,
        "",
        &ids
    )
    .fetch_all(db)
    .await
    .map(|res| {
        let hm: HashMap<_, _> = res
            .into_iter()
            .map(|x| record_to_images_with_authors_and_votes!(x))
            .map(|x| (x.0.id, x))
            .collect();
        ids.into_iter().map(|id| hm[&id].clone()).collect()
    })
}

pub async fn get_image_with_authors_and_votes_by_id(
    image_id: i64,
    curr_user_id: i64,
) -> Result<Option<(Image, User, ImageVotes)>, sqlx::Error> {
    let db = crate::DB_CONN.get().unwrap();
    get_images_with_authors_and_votes!(curr_user_id, r#"where i."id" = $2"#, "", image_id)
        .fetch_optional(db)
        .await
        .map(|x| x.map(|y| record_to_images_with_authors_and_votes!(y)))
}

pub async fn insert_image(image: &mut Image) -> Result<(), sqlx::Error> {
    let db = crate::DB_CONN.get().unwrap();
    image.id = sqlx::query!(
        r#"insert into "images" ("format", "title", "author", "timestamp") values ($1, $2, $3, $4) returning "id""#,
        image.format, image.title, image.author, image.timestamp
    )
    .fetch_one(db)
    .await?
    .id;
    Ok(())
}

pub async fn delete_image(id: i64) -> Result<(), sqlx::Error> {
    let db = crate::DB_CONN.get().unwrap();
    sqlx::query!(r#"delete from "images" where "id" = $1"#, id)
        .execute(db)
        .await?;
    Ok(())
}
