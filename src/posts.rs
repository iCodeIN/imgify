use std::collections::HashMap;

use worker::*;

use crate::{BUCKET, MAX_UPLOAD_BYTES};

pub async fn new(mut req: Request, ctx: RouteContext<()>) -> Result<Response> {
    if let Ok(bkt) = ctx.bucket(BUCKET) {
        let id = crate::randstr(10);
        let revoke = crate::randstr(64);
        let form = match req.form_data().await {
            Ok(val) => val,
            Err(e) => {
                return Response::error(&format!("Failed to get data from POST request: {e}"), 400)
            }
        };
        let files = if let Some(data) = form.get_all("data") {
            data
        } else {
            return Response::error("No \"data\" associated with upload!", 400);
        };
        if files.len() > 5 {
            return Response::error("Imagesets have a 5-file limit!", 400);
        }
        for item in files {
            match item {
                FormEntry::Field(_) => {
                    return Response::error("Text files are not supported!", 400)
                }
                FormEntry::File(f) => {
                    if f.size() > MAX_UPLOAD_BYTES {
                        return Response::error("File too large!", 400);
                    }
                    let data = f.bytes().await?;
                    if !infer::is_image(&data) {
                        return Response::error("Files must be images!", 400);
                    }
                    let mut metadata = HashMap::with_capacity(2);
                    metadata.insert("name".to_string(), f.name());
                    metadata.insert("revoke".to_string(), revoke.clone());
                    metadata.insert("type".to_string(), f.type_());
                    let filename = f.name();
                    let filename = urlencoding::encode(&filename);
                    match bkt
                        .put(format!("{id}/{}", filename), data)
                        .custom_metdata(metadata)
                        .execute()
                        .await
                    {
                        Ok(val) => val,
                        Err(e) => return Response::error(&format!("R2 error: {e}"), 500),
                    };
                }
            }
        }

        return Response::from_json(&serde_json::json!({"id": id, "revoke": revoke}));
    };
    Response::error("Account Misconfigured, no R2 instance found", 500)
}

pub async fn delete(mut _req: Request, ctx: RouteContext<()>) -> Result<Response> {
    if let Ok(bkt) = ctx.bucket(BUCKET) {
        let id = match ctx.param("id") {
            Some(val) => val,
            None => return Response::error("No imageset ID!", 400),
        };
        let token = match ctx.param("token") {
            Some(val) => val,
            None => return Response::error("No delete token!!", 400),
        };
        let objects = match bkt.list().prefix(id).execute().await {
            Ok(objs) => objs.objects(),
            Err(_) => return Response::error("Failed to list files in imageset!", 500),
        };
        for object in objects {
            console_log!("Deleting object {}", object.key());
            if let Some(correct_token) = object.custom_metadata()?.get("revoke") {
                console_log!("Correct token: {correct_token}. Provided token: {token}");
                if correct_token == token {
                    bkt.delete(object.key()).await.ok();
                }
            }
        }

        return Response::ok(format!("Deleted imageset {id}!"));
    }
    Response::error("Account Misconfigured, no CLOUDPASTE kv found", 500)
}
