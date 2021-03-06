Adventure
=========

[![crates.io][b/crates/i]][b/crates] [![docs.rs][b/docs/i]][b/docs]

[b/crates]: https://crates.io/crates/adventure
[b/crates/i]: https://meritbadge.herokuapp.com/adventure
[b/docs]: https://docs.rs/adventure/
[b/docs/i]: https://docs.rs/adventure/badge.svg

Provides general utilities for the web requests, like [exponential backoff][] and pagination.

[exponential backoff]: https://en.wikipedia.org/wiki/Exponential_backoff


Examples
--------

```rust
use std::sync::Arc;

use adventure::prelude::*;
use futures::prelude::*;

use adventure_rusoto_ecs::AwsEcs;
use rusoto_core::Region;
use rusoto_ecs::{EcsClient, ListServicesRequest};

fn main() {
    let client = EcsClient::new(Region::default());
    let req = ListServicesRequest {
        cluster: Some("MyEcsCluster".to_owned()),
        ..Default::default()
    };

    tokio::run(

        // prepare a request
        AwsEcs::from(req)
            // backoff if server error is occured
            .retry()
            // repeat to retrieve all results
            .paginate(Arc::new(client))

            // compatible with futures
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
```
