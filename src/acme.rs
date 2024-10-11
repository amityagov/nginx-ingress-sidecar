use instant_acme::{Account, Identifier, LetsEncrypt, NewAccount, NewOrder};
use serde::Serialize;

use crate::{
    nginx::{apply_operations, ServiceOperation},
    settings::NginxSettings,
    template::{render_template, Template},
};

#[derive(Serialize)]
pub struct AcmeTemplate {
    state: String,
    server_name: String,
    challenge_path: String,
    challenge: String,
}

impl AcmeTemplate {}

impl Template for AcmeTemplate {
    const NAME: &'static str = "acme";

    const TEMPLATE: &'static str = include_str!("../templates/acme.tmpl");
}

pub async fn challenge() -> anyhow::Result<()> {
    let (account, credentials) = Account::create(
        &NewAccount {
            contact: &[],
            terms_of_service_agreed: true,
            only_return_existing: false,
        },
        LetsEncrypt::Staging.url(),
        None,
    )
    .await?;
    println!(
        "account credentials:\n\n{}",
        serde_json::to_string_pretty(&credentials).unwrap()
    );

    let identifier = Identifier::Dns("flowwu.ru".to_string());

    let mut order = account
        .new_order(&NewOrder {
            identifiers: &[identifier],
        })
        .await?;

    let state = order.state();
    println!("order state: {:#?}", state);

    let authorizations = order.authorizations().await.unwrap();

    for authz in &authorizations {
        println!("{:?}", authz);

        println!("{:?}", authz.challenges);
    }

    Ok(())
}

async fn build_and_deploy_acme_server(_nginx: &NginxSettings) -> anyhow::Result<()> {
    let template = AcmeTemplate {
        state: todo!(),
        server_name: todo!(),
        challenge_path: todo!(),
        challenge: todo!(),
    };

    let content = render_template(&template)?;
    let operations = vec![ServiceOperation::Add];

    apply_operations(operations)?;

    Ok(())
}

async fn remove_acme_server() -> anyhow::Result<()> {
    let operations = vec![ServiceOperation::Remove];
    apply_operations(operations)?;
    Ok(())
}

#[tokio::test]
async fn test() {
    let result = challenge().await;
    println!("{:?}", result);
}
