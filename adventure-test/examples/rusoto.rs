use std::env::args;
use std::sync::Arc;

use adventure::prelude::*;
use adventure_rusoto_ecs::AwsEcs;
use futures::prelude::*;
use rusoto_core::Region;
use rusoto_ecs::{EcsClient, ListServicesRequest};

fn main() {
    let args: Vec<_> = args().take(2).collect();
    let cluster = args.get(1).cloned();

    let client = EcsClient::new(Region::default());
    let req = ListServicesRequest {
        cluster,
        ..Default::default()
    };

    let req = AwsEcs::from(req);

    #[cfg(feature = "backoff-tokio")]
    let req = req.retry();

    tokio::run(
        req.paginate(Arc::new(client))
            .try_for_each(|page| {
                for service in page.service_arns.unwrap_or_else(Vec::new) {
                    println!("{}", service);
                }
                future::ok(())
            })
            .or_else(|err| {
                eprintln!("Error occured: {}", err);
                future::ok(())
            })
            .compat(),
    );
}
