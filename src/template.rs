use serde::Serialize;
use tinytemplate::TinyTemplate;

pub trait Template {
    const NAME: &'static str;

    const TEMPLATE: &'static str;
}

pub fn render_template<T: Serialize + Template>(context: &T) -> anyhow::Result<String> {
    let mut tt = TinyTemplate::new();
    tt.add_template(T::NAME, T::TEMPLATE)?;
    Ok(tt.render(T::NAME, &context)?)
}
