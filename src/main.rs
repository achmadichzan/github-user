#![windows_subsystem = "windows"]
mod client;
mod models;
slint::include_modules!();

use std::env;
use std::sync::Arc;
use slint::Model;

fn main() -> anyhow::Result<()> {
    // Load .env variables
    dotenvy::dotenv().ok();
    let token: Option<String> = env::var("GITHUB_TOKEN")
        .ok()
        .filter(|t| !t.is_empty());

    // Background tokio runtime for async HTTP
    let rt = Arc::new(tokio::runtime::Runtime::new()?);

    // Create the UI
    let app = AppWindow::new()?;

    // =============================================
    //  CALLBACK: search-requested (List page)
    // =============================================
    {
        let app_weak = app.as_weak();
        let token = token.clone();
        let rt = rt.clone();

        app.on_search_requested(move |query| {
            let app_weak = app_weak.clone();
            let token = token.clone();
            let query = query.to_string();

            if let Some(app) = app_weak.upgrade() {
                app.set_is_searching(true);
                app.set_error_message("".into());
            }

            let app_weak_inner = app_weak.clone();

            rt.spawn(async move {
                let result = client::search_users(&query, token.as_deref()).await;

                match result {
                    Ok(users) => {
                        // Download all avatar thumbnails in parallel
                        let mut handles = Vec::new();
                        for user in &users {
                            let url = user.avatar_url.clone();
                            handles.push(tokio::spawn(download_avatar_pixels(url, 80)));
                        }

                        // Collect results
                        let mut items: Vec<(slint::SharedString, Option<(Vec<u8>, u32, u32)>)> =
                            Vec::new();
                        for (i, handle) in handles.into_iter().enumerate() {
                            let pixels = handle.await.ok().flatten();
                            let login: slint::SharedString = users[i].login.clone().into();
                            items.push((login, pixels));
                        }

                        let _ = slint::invoke_from_event_loop(move || {
                            if let Some(app) = app_weak_inner.upgrade() {
                                let model_items: Vec<UserItem> = items
                                    .into_iter()
                                    .map(|(login, pixels)| {
                                        let avatar = if let Some((px, w, h)) = pixels {
                                            let buf = slint::SharedPixelBuffer::<
                                                slint::Rgba8Pixel,
                                            >::clone_from_slice(&px, w, h);
                                            slint::Image::from_rgba8(buf)
                                        } else {
                                            slint::Image::default()
                                        };
                                        UserItem { login, avatar }
                                    })
                                    .collect();

                                let model = std::rc::Rc::new(slint::VecModel::from(model_items));
                                app.set_user_list(model.into());
                                app.set_is_searching(false);
                            }
                        });
                    }
                    Err(e) => {
                        let msg: slint::SharedString = format!("Error: {e}").into();
                        let _ = slint::invoke_from_event_loop(move || {
                            if let Some(app) = app_weak_inner.upgrade() {
                                app.set_error_message(msg);
                                app.set_is_searching(false);
                            }
                        });
                    }
                }
            });
        });
    }

    // =============================================
    //  CALLBACK: user-selected (List → Detail)
    // =============================================
    {
        let app_weak = app.as_weak();
        let token = token.clone();
        let rt = rt.clone();

        app.on_user_selected(move |index| {
            let app_weak = app_weak.clone();
            let token = token.clone();

            // Get the login from the list model
            let login = {
                let Some(app) = app_weak.upgrade() else { return };
                let list = app.get_user_list();
                let row = list.row_data(index as usize);
                match row {
                    Some(item) => item.login.to_string(),
                    None => return,
                }
            };

            // Navigate to detail page and show loading
            if let Some(app) = app_weak.upgrade() {
                app.set_current_page(1);
                app.set_is_loading(true);
                app.set_error_message("".into());
                app.set_login_name("".into());
            }

            let app_weak_inner = app_weak.clone();

            rt.spawn(async move {
                let result = client::fetch_user(&login, token.as_deref()).await;

                match result {
                    Ok(user) => {
                        let avatar_pixels = download_avatar_pixels(user.avatar_url.clone(), 256).await;

                        let login: slint::SharedString = user.login.into();
                        let name: slint::SharedString =
                            user.name.unwrap_or_default().into();
                        let bio: slint::SharedString =
                            user.bio.unwrap_or_default().into();
                        let repos: slint::SharedString =
                            user.public_repos.to_string().into();
                        let followers: slint::SharedString =
                            user.followers.to_string().into();
                        let following: slint::SharedString =
                            user.following.to_string().into();
                        let profile_url: slint::SharedString = user.html_url.into();
                        let created_at: slint::SharedString = user.created_at.into();

                        let _ = slint::invoke_from_event_loop(move || {
                            if let Some(app) = app_weak_inner.upgrade() {
                                app.set_login_name(login);
                                app.set_display_name(name);
                                app.set_bio(bio);
                                app.set_repos(repos);
                                app.set_followers(followers);
                                app.set_following(following);
                                app.set_profile_url(profile_url);
                                app.set_created_at(created_at);

                                if let Some((pixels, w, h)) = avatar_pixels {
                                    let buf = slint::SharedPixelBuffer::<
                                        slint::Rgba8Pixel,
                                    >::clone_from_slice(&pixels, w, h);
                                    app.set_avatar(slint::Image::from_rgba8(buf));
                                }

                                app.set_is_loading(false);
                            }
                        });
                    }
                    Err(e) => {
                        let msg: slint::SharedString = format!("Error: {e}").into();
                        let _ = slint::invoke_from_event_loop(move || {
                            if let Some(app) = app_weak_inner.upgrade() {
                                app.set_error_message(msg);
                                app.set_is_loading(false);
                            }
                        });
                    }
                }
            });
        });
    }

    // Trigger initial search for "a" on startup
    app.invoke_search_requested(app.get_search_query());

    // Run the Slint event loop
    app.run()?;

    Ok(())
}

/// Downloads avatar image bytes and decodes them into raw RGBA pixels.
async fn download_avatar_pixels(url: String, size: u32) -> Option<(Vec<u8>, u32, u32)> {
    // Use a specified size
    let sized_url = if url.contains('?') {
        format!("{url}&s={size}")
    } else {
        format!("{url}?s={size}")
    };

    let bytes = reqwest::get(&sized_url).await.ok()?.bytes().await.ok()?;
    let dynamic_image = image::load_from_memory(&bytes).ok()?;
    let rgba = dynamic_image.to_rgba8();
    let (w, h) = rgba.dimensions();

    Some((rgba.into_raw(), w, h))
}
