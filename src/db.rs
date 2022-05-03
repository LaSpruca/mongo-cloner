use mongodb::error::{Error as MongoError, Result as MongoResult};
use mongodb::Client;
use tokio::runtime::Runtime;
use tokio::sync::mpsc::{channel, Sender};
use tracing::{debug, info};
use url::Url;

#[derive(Debug)]
pub struct DbName {
    pub name: String,
    pub rename: String,
}

#[derive(Debug)]
pub struct DbCollection {
    pub name: String,
    pub rename: String,
    pub selected: bool,
}

#[derive(Debug)]
pub struct Db {
    pub db_name: DbName,
    pub collections: Vec<DbCollection>,
}

impl From<(String, Vec<String>)> for Db {
    fn from((name, collections): (String, Vec<String>)) -> Self {
        Self {
            db_name: DbName {
                rename: name.clone(),
                name,
            },
            collections: collections
                .into_iter()
                .map(|x| DbCollection {
                    name: x.clone(),
                    rename: x,
                    selected: true,
                })
                .collect(),
        }
    }
}

type DbClientSender = Sender<(Request, Sender<Response>)>;
type DbClientChannel = (Request, Sender<Response>);

#[derive(Debug)]
enum Request {
    ListCollections,
    Exit,
}

#[derive(Debug)]
enum Response {
    MongoError(MongoError),
    Collections(Vec<(String, Vec<String>)>),
}

pub struct DbClient {
    tx: DbClientSender,
}

impl DbClient {
    pub fn new(url: &Url, rt: &Runtime) -> MongoResult<Self> {
        info!("Connecting to {url}");
        let client = rt.block_on(Client::with_uri_str(&format!("{url}")))?;
        let (tx, mut rx) = channel::<DbClientChannel>(5);

        rt.spawn(async move {
            'command_loop: while let Some((request, sender)) = rx.recv().await {
                match request {
                    Request::ListCollections => {
                        match client.list_database_names(None, None).await {
                            Ok(databases) => {
                                let mut response = vec![];
                                for database in databases {
                                    match client
                                        .database(&database)
                                        .list_collection_names(None)
                                        .await
                                    {
                                        Ok(collections) => {
                                            response.push((database, collections));
                                        }
                                        Err(ex) => {
                                            sender.send(Response::MongoError(ex)).await.unwrap();
                                            continue 'command_loop;
                                        }
                                    };
                                }
                                sender.send(Response::Collections(response)).await.unwrap();
                            }
                            Err(ex) => {
                                sender.send(Response::MongoError(ex)).await.unwrap();
                            }
                        };
                    }
                    Request::Exit => {
                        rx.close();
                        break;
                    }
                }
            }
        });

        Ok(Self { tx })
    }

    pub async fn get_collections(&self) -> MongoResult<Vec<(String, Vec<String>)>> {
        let (tx, mut rx) = channel(1);
        self.tx.send((Request::ListCollections, tx)).await.unwrap();

        if let Some(result) = rx.recv().await {
            debug!("{result:?}");
            #[allow(unreachable_patterns)]
            return match result {
                Response::MongoError(ex) => Err(ex),
                Response::Collections(collections) => Ok(collections),
                _ => unreachable!(),
            };
        }

        unreachable!()
    }
}

impl Drop for DbClient {
    fn drop(&mut self) {
        let (tx, mut rx) = channel(1);
        self.tx.blocking_send((Request::Exit, tx)).unwrap();
        while rx.blocking_recv().is_some() {}
    }
}
