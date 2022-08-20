use worker::*;

use crate::{error, BUCKET};

pub async fn template(_req: Request, ctx: RouteContext<()>) -> Result<Response> {
    if let Ok(bkt) = ctx.bucket(BUCKET) {
        let key = match ctx.param("id") {
            Some(val) => val,
            None => return error("No Image ID!", 400, true),
        };
        let files = match bkt.list().prefix(key).execute().await {
            Ok(val) => val.objects(),
            Err(e) => return error(&format!("R2 Error: {}", e), 500, true),
        };

        if files.is_empty() {
            return error("Imageset not found!", 404, true);
        }

        let mut context = tera::Context::new();
        context.insert("id", key);
        context.insert(
            "images",
            &files
                .iter()
                .map(|f| f.key().split_once('/').unwrap_or(("", "")).1.to_string())
                .collect::<Vec<String>>(),
        );
        if let Ok(page) = tera::Tera::one_off(include_str!("html/img.html"), &context, true) {
            return Response::from_html(page);
        }

        return error(
            "Templating failed! (this is a bug, github.com/randomairborne/imgify)",
            500,
            true,
        );
    }
    Response::error("Account Misconfigured, no imgify kv found", 500)
}

pub async fn raw(_req: Request, ctx: RouteContext<()>) -> Result<Response> {
    if let Ok(bkt) = ctx.bucket(BUCKET) {
        let id = match ctx.param("id") {
            Some(val) => val,
            None => return Response::error("No image ID!", 400),
        };
        let name = match ctx.param("name") {
            Some(val) => val,
            None => return Response::error("No image name!", 400),
        };
        let maybe_value = match bkt.get(format!("{id}/{name}")).execute().await {
            Ok(val) => val,
            Err(e) => return error(&format!("R2 Error: {}", e), 500, true),
        };

        if let Some(value) = maybe_value {
            return Response::from_bytes(value.body().unwrap().bytes().await.unwrap());
        }
        return Response::error("Image Not Found!", 404);
    }
    Response::error("Account Misconfigured, no imgify kv found", 500)
}
pub mod style {
    use worker::*;
    pub fn main(_req: Request, _ctx: RouteContext<()>) -> Result<Response> {
        let mut headers = Headers::new();
        headers.append("Content-Type", "text/css")?;
        Ok(Response::ok(include_str!("html/main.css"))?.with_headers(headers))
    }
    pub fn submit(_req: Request, _ctx: RouteContext<()>) -> Result<Response> {
        let mut headers = Headers::new();
        headers.append("Content-Type", "text/css")?;
        Ok(Response::ok(include_str!("html/index.css"))?.with_headers(headers))
    }
}
