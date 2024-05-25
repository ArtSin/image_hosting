#![cfg(feature = "ssr")]

use crate::image_votes::ImageVotes;

pub async fn get_image_votes(curr_user_id: i64, image_id: i64) -> Result<ImageVotes, sqlx::Error> {
    let db = crate::DB_CONN.get().unwrap();
    sqlx::query!(
        r#"
        select
            (coalesce(sum(case when iv."upvote" then 1 else -1 end), 0)) as "rating!",
            iv_curr."upvote" as "curr_user_upvote?"
        from
            (select "user_id", "upvote" from "images_votes" where "images_votes"."image_id" = $2) as iv
        left join
            "images_votes" iv_curr on iv_curr."image_id" = $2 and iv_curr."user_id" = $1
        group by
            iv_curr."upvote"
        "#,
        curr_user_id,
        image_id
    )
    .fetch_optional(db)
    .await
    .map(|x| {
        x.map(|y| ImageVotes {
            image_id,
            rating: y.rating,
            curr_user_upvote: y.curr_user_upvote,
        })
        .unwrap_or_else(|| ImageVotes {
            image_id,
            rating: 0,
            curr_user_upvote: None
        })
    })
}

pub async fn insert_image_vote(
    image_id: i64,
    user_id: i64,
    upvote: bool,
) -> Result<(), sqlx::Error> {
    let db = crate::DB_CONN.get().unwrap();
    sqlx::query!(
        r#"insert into "images_votes" ("image_id", "user_id", "upvote") values ($1, $2, $3)"#,
        image_id,
        user_id,
        upvote
    )
    .execute(db)
    .await?;
    Ok(())
}

pub async fn delete_image_vote(image_id: i64, user_id: i64) -> Result<(), sqlx::Error> {
    let db = crate::DB_CONN.get().unwrap();
    sqlx::query!(
        r#"delete from "images_votes" where "image_id" = $1 and "user_id" = $2"#,
        image_id,
        user_id
    )
    .execute(db)
    .await?;
    Ok(())
}
