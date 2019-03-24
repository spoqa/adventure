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

    #[cfg(all(feature = "futures", not(feature = "std-future")))]
    tokio::run(
        AwsEcs::from(req)
            .retry()
            .paginate(Arc::new(client))
            .for_each(|page| {
                for service in page.service_arns.unwrap_or_else(Vec::new) {
                    println!("{}", service);
                }
                Ok(())
            })
            .or_else(|err| {
                eprintln!("Error occured: {}", err);
                Ok(())
            }),
    );
}
