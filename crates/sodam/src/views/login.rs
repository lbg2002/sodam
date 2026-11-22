//! 登录二维码弹层。

use super::*;
/// 登录 modal：居中卡片 + 二维码 + 扫码提示 + 关闭。
pub fn login_modal(root: &Root, cx: &mut Context<Root>) -> gpui::AnyElement {
    use gpui::deferred;

    let card = match &root.login {
        LoginState::Idle | LoginState::Loading => div()
            .w(px(320.0))
            .flex()
            .flex_col()
            .items_center()
            .gap(px(theme::space::LG))
            .p(px(theme::space::XL))
            .rounded(px(theme::radius::CARD))
            .bg(theme::surface_elevated())
            .border_1()
            .border_color(theme::border())
            .shadow_lg()
            .child(crate::ui::spinner::loading_state(
                root.tr("正在创建二维码…"),
            ))
            .child(close_button(root.language, cx))
            .into_any_element(),
        LoginState::Waiting {
            matrix,
            status,
            token,
        } => div()
            .w(px(360.0))
            .flex()
            .flex_col()
            .items_center()
            .gap(px(theme::space::LG))
            .p(px(theme::space::XL))
            .rounded(px(theme::radius::CARD))
            .bg(theme::surface_elevated())
            .border_1()
            .border_color(theme::border())
            .shadow_lg()
            .child(
                div()
                    .text_size(theme::Text::Title.size())
                    .font_weight(gpui::FontWeight::BOLD)
                    .text_color(theme::text())
                    .child(root.tr("扫码登录")),
            )
            .child(
                div()
                    .p(px(theme::space::SM))
                    .rounded(px(theme::radius::CARD))
                    .bg(gpui::white())
                    .child(qr_canvas(matrix.clone())),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .items_center()
                    .gap(px(theme::space::XS))
                    .child(
                        div()
                            .text_size(theme::Text::Small.size())
                            .text_color(theme::text())
                            .child(root.tr("请使用汽水音乐 App 扫码登录")),
                    )
                    .child(
                        div()
                            .text_size(theme::Text::Tiny.size())
                            .text_color(theme::text_muted())
                            .child(status.clone()),
                    )
                    .child(
                        div()
                            .text_size(theme::Text::Tiny.size())
                            .text_color(theme::text_faint())
                            .child(root.language.textf(
                                root.tr("会话 {}…"),
                                &[token.chars().take(10).collect::<String>()],
                            )),
                    ),
            )
            .child(close_button(root.language, cx))
            .into_any_element(),
        LoginState::LoggedIn(_) | LoginState::Failed(_) => div()
            .w(px(320.0))
            .flex()
            .flex_col()
            .items_center()
            .gap(px(theme::space::LG))
            .p(px(theme::space::XL))
            .rounded(px(theme::radius::CARD))
            .bg(theme::surface_elevated())
            .border_1()
            .border_color(theme::border())
            .shadow_lg()
            .child(
                div()
                    .text_size(theme::Text::Small.size())
                    .text_color(theme::text_muted())
                    .child(match &root.login {
                        LoginState::Failed(err) => root
                            .language
                            .textf("登录失败：{err}", std::slice::from_ref(err)),
                        _ => root.tr("已登录").to_string(),
                    }),
            )
            .child(close_button(root.language, cx))
            .into_any_element(),
    };

    let scrim = div()
        .id("login-modal")
        .absolute()
        .size_full()
        .flex()
        .items_center()
        .justify_center()
        .bg(gpui::black())
        .opacity(0.55)
        .on_mouse_down(
            gpui::MouseButton::Left,
            cx.listener(|root, _event, _window, cx| {
                root.login_modal_open = false;
                cx.notify();
            }),
        );

    deferred(
        div()
            .absolute()
            .size_full()
            .flex()
            .items_center()
            .justify_center()
            .child(scrim)
            .child(card),
    )
    .with_priority(10)
    .into_any_element()
}

fn close_button(language: crate::ui::i18n::Language, cx: &mut Context<Root>) -> impl IntoElement {
    div()
        .id("login-close")
        .flex()
        .flex_row()
        .items_center()
        .gap(px(theme::space::XS))
        .px(px(theme::space::MD))
        .h(px(theme::size::CONTROL_SM))
        .rounded(px(theme::radius::PILL))
        .bg(theme::surface_hover())
        .cursor_pointer()
        .hover(|style| style.bg(theme::surface_selected()))
        .text_size(theme::Text::Tiny.size())
        .text_color(theme::text_muted())
        .on_click(cx.listener(|root, _event: &ClickEvent, _window, cx| {
            root.login_modal_open = false;
            cx.notify();
        }))
        .child(language.text("关闭"))
}

fn mask_secret_preview(value: &str) -> String {
    let chars: Vec<char> = value.trim().chars().collect();
    if chars.is_empty() {
        return String::new();
    }
    if chars.len() <= 8 {
        return format!("{} ({})", "•".repeat(chars.len()), chars.len());
    }
    let prefix: String = chars[..4].iter().collect();
    let suffix: String = chars[chars.len() - 4..].iter().collect();
    format!("{prefix}••••{suffix} ({})", chars.len())
}

fn migu_text_field(
    root: &Root,
    window: &Window,
    id: &'static str,
    field: crate::app::MiguInputField,
    placeholder: String,
    value_display: String,
    cx: &mut Context<Root>,
) -> gpui::AnyElement {
    let active = root.migu_active_field == field && root.migu_input_focus.is_focused(window);
    let has_text = !value_display.is_empty();
    let shown = if has_text {
        format!("{value_display}{}", if active { "▌" } else { "" })
    } else if active {
        "▌".to_string()
    } else {
        placeholder
    };
    let input_focus = root.migu_input_focus.clone();
    let input_entity = cx.entity();

    div()
        .id(id)
        .relative()
        .when(active, |this| this.track_focus(&root.migu_input_focus))
        .flex()
        .flex_row()
        .items_center()
        .w_full()
        .h(px(38.0))
        .px(px(theme::space::MD))
        .rounded(px(theme::radius::ROW))
        .bg(theme::surface())
        .border_1()
        .border_color(if active {
            theme::accent()
        } else {
            theme::border()
        })
        .text_size(theme::Text::Small.size())
        .text_color(if has_text || active {
            theme::text()
        } else {
            theme::text_muted()
        })
        .cursor_text()
        .on_mouse_down(
            MouseButton::Left,
            cx.listener(move |root, _event, window, cx| {
                root.migu_active_field = field;
                root.migu_input_marked_range = None;
                window.focus(&root.migu_input_focus, cx);
                cx.notify();
            }),
        )
        .on_key_down(cx.listener(|root, event: &KeyDownEvent, _window, cx| {
            let key = event.keystroke.key.as_str();
            let paste = (event.keystroke.modifiers.control || event.keystroke.modifiers.platform)
                && key.eq_ignore_ascii_case("v");
            if paste {
                cx.stop_propagation();
                if let Some(clip) = cx
                    .read_from_clipboard()
                    .and_then(|item| item.text())
                    .map(|t| t.trim().to_string())
                    .filter(|t| !t.is_empty())
                {
                    root.migu_active_input_mut().push_str(&clip);
                    cx.notify();
                }
                return;
            }
            match key {
                "backspace" => {
                    cx.stop_propagation();
                    let _ = root.migu_active_input_mut().pop();
                    cx.notify();
                }
                "enter" => {
                    cx.stop_propagation();
                    match root.migu_login_tab {
                        crate::app::MiguLoginTab::Cookie => root.submit_migu_cookie(cx),
                        crate::app::MiguLoginTab::Sim => root.start_migu_sim_login(cx),
                        crate::app::MiguLoginTab::Sms => root.submit_migu_sms_login(cx),
                    }
                }
                _ => {}
            }
        }))
        .when(active, |this| {
            this.child(
                canvas(
                    |_bounds, _window, _cx| {},
                    move |bounds, _state, window, cx| {
                        window.handle_input(
                            &input_focus,
                            gpui::ElementInputHandler::new(bounds, input_entity.clone()),
                            cx,
                        );
                    },
                )
                .absolute()
                .size_full(),
            )
        })
        .child(div().flex_1().min_w(px(0.0)).truncate().child(shown))
        .when(has_text, |this| {
            this.child(
                div()
                    .id((id, 1usize))
                    .flex_none()
                    .flex()
                    .items_center()
                    .justify_center()
                    .size(px(22.0))
                    .rounded(px(theme::radius::PILL))
                    .cursor_pointer()
                    .hover(|style| style.bg(theme::surface_hover()))
                    .on_mouse_down(MouseButton::Left, |_event, _window, cx| {
                        cx.stop_propagation();
                    })
                    .on_click(cx.listener(move |root, _event: &ClickEvent, _window, cx| {
                        match field {
                            crate::app::MiguInputField::Cookie => root.migu_cookie_input.clear(),
                            crate::app::MiguInputField::Phone => root.migu_phone_input.clear(),
                            crate::app::MiguInputField::Code => root.migu_code_input.clear(),
                        }
                        root.migu_input_marked_range = None;
                        cx.notify();
                    }))
                    .child(
                        svg()
                            .path(icons::path("x"))
                            .size(px(12.0))
                            .text_color(theme::text_muted()),
                    ),
            )
        })
        .into_any_element()
}

fn migu_tab_pill(
    root: &Root,
    id: &'static str,
    tab: crate::app::MiguLoginTab,
    label: &'static str,
    cx: &mut Context<Root>,
) -> gpui::AnyElement {
    let selected = root.migu_login_tab == tab;
    div()
        .id(id)
        .flex_1()
        .h(px(32.0))
        .flex()
        .items_center()
        .justify_center()
        .rounded(px(theme::radius::PILL))
        .bg(if selected {
            theme::accent()
        } else {
            theme::surface()
        })
        .text_size(theme::Text::Tiny.size())
        .font_weight(if selected {
            gpui::FontWeight::SEMIBOLD
        } else {
            gpui::FontWeight::MEDIUM
        })
        .text_color(if selected {
            theme::accent_foreground()
        } else {
            theme::text_muted()
        })
        .cursor_pointer()
        .hover(|style| {
            if selected {
                style.opacity(0.92)
            } else {
                style.bg(theme::surface_hover())
            }
        })
        .on_click(cx.listener(move |root, _event: &ClickEvent, window, cx| {
            root.migu_login_tab = tab;
            root.migu_active_field = match tab {
                crate::app::MiguLoginTab::Cookie => crate::app::MiguInputField::Cookie,
                crate::app::MiguLoginTab::Sim | crate::app::MiguLoginTab::Sms => {
                    crate::app::MiguInputField::Phone
                }
            };
            root.migu_login_status.clear();
            window.focus(&root.migu_input_focus, cx);
            cx.notify();
        }))
        .child(root.tr(label))
        .into_any_element()
}

/// 咪咕音乐会员登录弹层：支持 Cookie / Token 导入（含剪贴板一键读取）、移动 SIM 快捷认证、短信动态码。
pub fn migu_login_modal(root: &Root, window: &Window, cx: &mut Context<Root>) -> gpui::AnyElement {
    use crate::app::{MiguInputField, MiguLoginTab};
    use gpui::deferred;

    let tab_body: gpui::AnyElement = match root.migu_login_tab {
        MiguLoginTab::Cookie => div()
            .flex()
            .flex_col()
            .gap(px(theme::space::MD))
            .child(
                div()
                    .p(px(theme::space::MD))
                    .rounded(px(theme::radius::ROW))
                    .bg(theme::surface())
                    .text_size(theme::Text::Tiny.size())
                    .text_color(theme::text_muted())
                    .child(root.tr("在浏览器登录 music.migu.cn 后，从开发者工具复制请求 Cookie、cURL 或 pacmtoken 即可解锁 VIP 与无损音质。")),
            )
            .child(migu_text_field(
                root,
                window,
                "migu-cookie-input",
                MiguInputField::Cookie,
                root.tr("点击后 Ctrl+V 粘贴 Cookie / pacmtoken / cURL…")
                    .to_string(),
                mask_secret_preview(&root.migu_cookie_input),
                cx,
            ))
            .child(
                div()
                    .flex()
                    .flex_row()
                    .flex_wrap()
                    .gap(px(theme::space::SM))
                    .child(
                        div()
                            .id("migu-open-web")
                            .px(px(theme::space::MD))
                            .h(px(34.0))
                            .flex()
                            .items_center()
                            .rounded(px(theme::radius::ROW))
                            .bg(theme::surface())
                            .border_1()
                            .border_color(theme::border())
                            .text_size(theme::Text::Small.size())
                            .text_color(theme::text())
                            .cursor_pointer()
                            .hover(|style| style.bg(theme::surface_hover()))
                            .on_click(cx.listener(|root, _event: &ClickEvent, _window, cx| {
                                root.open_migu_web(cx);
                            }))
                            .child(root.tr("打开咪咕网页版")),
                    )
                    .child(
                        div()
                            .id("migu-paste-clipboard")
                            .px(px(theme::space::MD))
                            .h(px(34.0))
                            .flex()
                            .items_center()
                            .rounded(px(theme::radius::ROW))
                            .bg(theme::surface_elevated())
                            .border_1()
                            .border_color(theme::border())
                            .text_size(theme::Text::Small.size())
                            .text_color(theme::text())
                            .cursor_pointer()
                            .hover(|style| style.bg(theme::surface_hover()))
                            .on_click(cx.listener(|root, _event: &ClickEvent, _window, cx| {
                                root.paste_migu_cookie_from_clipboard(cx);
                            }))
                            .child(root.tr("读取剪贴板并登录")),
                    )
                    .child(
                        div()
                            .id("migu-submit-cookie")
                            .flex_1()
                            .min_w(px(110.0))
                            .px(px(theme::space::MD))
                            .h(px(34.0))
                            .flex()
                            .items_center()
                            .justify_center()
                            .rounded(px(theme::radius::ROW))
                            .bg(theme::accent())
                            .text_size(theme::Text::Small.size())
                            .font_weight(gpui::FontWeight::SEMIBOLD)
                            .text_color(theme::accent_foreground())
                            .cursor_pointer()
                            .hover(|style| style.opacity(0.9))
                            .on_click(cx.listener(|root, _event: &ClickEvent, _window, cx| {
                                root.submit_migu_cookie(cx);
                            }))
                            .child(root.tr("验证并登录")),
                    ),
            )
            .into_any_element(),
        MiguLoginTab::Sim => div()
            .flex()
            .flex_col()
            .gap(px(theme::space::MD))
            .child(
                div()
                    .p(px(theme::space::MD))
                    .rounded(px(theme::radius::ROW))
                    .bg(theme::surface())
                    .text_size(theme::Text::Tiny.size())
                    .text_color(theme::text_muted())
                    .child(root.tr("仅支持中国移动号码：点击发送后，请在手机端收到的 SIM 认证弹窗内确认登录。")),
            )
            .child(migu_text_field(
                root,
                window,
                "migu-sim-phone-input",
                MiguInputField::Phone,
                root.tr("输入 11 位中国移动手机号").to_string(),
                root.migu_phone_input.clone(),
                cx,
            ))
            .child(
                div()
                    .id("migu-sim-send")
                    .w_full()
                    .h(px(36.0))
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded(px(theme::radius::ROW))
                    .bg(theme::accent())
                    .text_size(theme::Text::Small.size())
                    .font_weight(gpui::FontWeight::SEMIBOLD)
                    .text_color(theme::accent_foreground())
                    .cursor_pointer()
                    .hover(|style| style.opacity(0.9))
                    .on_click(cx.listener(|root, _event: &ClickEvent, _window, cx| {
                        root.start_migu_sim_login(cx);
                    }))
                    .child(if root.migu_login_busy {
                        root.tr("等待手机确认…")
                    } else {
                        root.tr("发送 SIM 认证弹窗")
                    }),
            )
            .into_any_element(),
        MiguLoginTab::Sms => div()
            .flex()
            .flex_col()
            .gap(px(theme::space::MD))
            .child(migu_text_field(
                root,
                window,
                "migu-sms-phone-input",
                MiguInputField::Phone,
                root.tr("输入 11 位手机号").to_string(),
                root.migu_phone_input.clone(),
                cx,
            ))
            .child(
                div()
                    .flex()
                    .flex_row()
                    .gap(px(theme::space::SM))
                    .child(div().flex_1().child(migu_text_field(
                        root,
                        window,
                        "migu-sms-code-input",
                        MiguInputField::Code,
                        root.tr("输入短信验证码").to_string(),
                        root.migu_code_input.clone(),
                        cx,
                    )))
                    .child(
                        div()
                            .id("migu-sms-send-code")
                            .flex_none()
                            .px(px(theme::space::MD))
                            .h(px(38.0))
                            .flex()
                            .items_center()
                            .rounded(px(theme::radius::ROW))
                            .bg(theme::surface())
                            .border_1()
                            .border_color(theme::border())
                            .text_size(theme::Text::Small.size())
                            .text_color(theme::text())
                            .cursor_pointer()
                            .hover(|style| style.bg(theme::surface_hover()))
                            .on_click(cx.listener(|root, _event: &ClickEvent, _window, cx| {
                                root.send_migu_sms_code(cx);
                            }))
                            .child(root.tr("获取验证码")),
                    ),
            )
            .child(
                div()
                    .id("migu-sms-submit")
                    .w_full()
                    .h(px(36.0))
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded(px(theme::radius::ROW))
                    .bg(theme::accent())
                    .text_size(theme::Text::Small.size())
                    .font_weight(gpui::FontWeight::SEMIBOLD)
                    .text_color(theme::accent_foreground())
                    .cursor_pointer()
                    .hover(|style| style.opacity(0.9))
                    .on_click(cx.listener(|root, _event: &ClickEvent, _window, cx| {
                        root.submit_migu_sms_login(cx);
                    }))
                    .child(root.tr("验证并登录")),
            )
            .into_any_element(),
    };

    let card = div()
        .id("migu-login-card")
        .w(px(460.0))
        .max_w(px(500.0))
        .flex()
        .flex_col()
        .gap(px(theme::space::LG))
        .p(px(theme::space::XL))
        .rounded(px(theme::radius::CARD))
        .bg(theme::surface_elevated())
        .border_1()
        .border_color(theme::border())
        .shadow_lg()
        .on_mouse_down(MouseButton::Left, |_event, _window, cx| {
            cx.stop_propagation();
        })
        .child(
            div()
                .flex()
                .flex_row()
                .items_center()
                .justify_between()
                .child(
                    div()
                        .text_size(theme::Text::Title.size())
                        .font_weight(gpui::FontWeight::BOLD)
                        .text_color(theme::text())
                        .child(root.tr("登录咪咕音乐会员")),
                )
                .child(
                    div()
                        .id("migu-login-close")
                        .flex()
                        .items_center()
                        .justify_center()
                        .size(px(28.0))
                        .rounded(px(theme::radius::PILL))
                        .cursor_pointer()
                        .hover(|style| style.bg(theme::surface_hover()))
                        .on_click(cx.listener(|root, _event: &ClickEvent, _window, cx| {
                            root.close_migu_login_modal(cx);
                        }))
                        .child(
                            svg()
                                .path(icons::path("x"))
                                .size(px(theme::ICON_SM))
                                .text_color(theme::text_muted()),
                        ),
                ),
        )
        .child(
            div()
                .flex()
                .flex_row()
                .gap(px(theme::space::XS))
                .p(px(4.0))
                .rounded(px(theme::radius::PILL))
                .bg(theme::surface())
                .child(migu_tab_pill(
                    root,
                    "migu-tab-cookie",
                    MiguLoginTab::Cookie,
                    "Cookie 导入",
                    cx,
                ))
                .child(migu_tab_pill(
                    root,
                    "migu-tab-sim",
                    MiguLoginTab::Sim,
                    "SIM 快捷认证",
                    cx,
                ))
                .child(migu_tab_pill(
                    root,
                    "migu-tab-sms",
                    MiguLoginTab::Sms,
                    "短信验证码",
                    cx,
                )),
        )
        .child(tab_body)
        .when(!root.migu_login_status.is_empty(), |this| {
            this.child(
                div()
                    .text_size(theme::Text::Tiny.size())
                    .text_color(theme::accent())
                    .child(root.migu_login_status.clone()),
            )
        });

    let scrim = div()
        .id("migu-login-scrim")
        .absolute()
        .size_full()
        .bg(gpui::black())
        .opacity(0.55)
        .on_mouse_down(
            MouseButton::Left,
            cx.listener(|root, _event, _window, cx| {
                root.close_migu_login_modal(cx);
            }),
        );

    deferred(
        div()
            .absolute()
            .size_full()
            .flex()
            .items_center()
            .justify_center()
            .child(scrim)
            .child(card),
    )
    .with_priority(10)
    .into_any_element()
}
