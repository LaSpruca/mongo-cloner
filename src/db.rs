use futures::stream::TryStreamExt;
use mongodb::{
    bson::Document,
    error::{Error as MongoError, Result as MongoResult},
    Client,
};
use tokio::{
    runtime::Runtime,
    sync::{mpsc, oneshot},
};
use tracing::{debug, info};
use url::Url;

/// Used for storing a database's name and what it should be renamed to
#[derive(Debug, Clone)]
pub struct DbName {
    /// The databases name
    pub name: String,
    /// What it should be renamed to
    pub rename: String,
}

/// Used for storing a collection's name and what it should be renamed to and weather or not it
/// should be included in the move
#[derive(Debug, Clone)]
pub struct DbCollection {
    /// The name of the database
    pub name: String,
    /// What it should be renamed to
    pub rename: String,
    /// Weather or not it should be moved
    pub selected: bool,
}

/// A Database and it's collections
#[derive(Debug, Clone)]
pub struct Db {
    /// The Db's name
    pub db_name: DbName,
    /// The collections
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

// Convenience types
type DbClientChannel = (Request, oneshot::Sender<Response>);
type DbClientSender = mpsc::Sender<DbClientChannel>;

/// All the requests that can be made to a database
#[derive(Debug)]
enum Request {
    /// Get the names of all of the databases and collections in a cluster
    ListCollections,
    /// Upload a set of documents to a database with the given name
    UploadCollection {
        db: String,
        collection: String,
        documents: Vec<Document>,
    },
    /// Download a given collection
    DownloadCollection { db: String, collection: String },
}

#[derive(Debug)]
enum Response {
    /// Response type for if there is an error
    MongoError(MongoError),
    /// Successful  result of [`Request::ListCollections`](Request::ListCollections)
    Collections(Vec<(String, Vec<String>)>),
    /// Successful  result of [`Request::DownloadCollection`](Request::DownloadCollection)
    DownloadedDocuments(Vec<Document>),
    /// Successful result of [`Request::UploadCollection`](Request::UploadCollection)
    UploadSuccess,
}

/// A connection to a cluster
pub struct ClusterClient {
    /// Channel for sending requests to the loop
    tx: DbClientSender,
}

impl ClusterClient {
    /// Creates a new connection and spawns a request handler loop
    pub fn new(url: &Url, rt: &Runtime) -> MongoResult<Self> {
        info!("Connecting to {url}");
        let client = rt.block_on(Client::with_uri_str(&format!("{url}")))?;
        let (tx, mut rx) = mpsc::channel::<DbClientChannel>(1024);

        rt.spawn(async move {
            let thread = tokio::runtime::Handle::current();

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
                                            sender.send(Response::MongoError(ex)).expect("Sender for `ClusterClient::get_collections` dropped");
                                            continue 'command_loop;
                                        }
                                    };
                                }
                                sender
                                    .send(Response::Collections(response))
                                    .expect("Sender for `ClusterClient::get_collections` dropped");
                            }
                            Err(ex) => {
                                sender.send(Response::MongoError(ex)).expect("Sender for `ClusterClient::get_collections` dropped");
                            }
                        };
                    }
                    Request::UploadCollection {
                        collection,
                        db,
                        documents,
                    } => {
                        let client = client.clone();
                        thread.spawn(async move {
                            debug!("Uploading collection {db}.{collection}");
                            match client
                                .database(&db)
                                .collection(&collection)
                                .insert_many(documents, None)
                                .await
                            {
                                Ok(_) => sender.send(Response::UploadSuccess),
                                Err(ex) => sender.send(Response::MongoError(ex)),
                            }
                                .expect("Sender for `ClusterClient::upload_collection` dropped");
                            debug!("Uploaded collection {db}.{collection}");
                        });
                    }
                    Request::DownloadCollection { collection, db } => {
                        debug!("Getting collection {db}.{collection}");
                        let client = client.clone();
                        thread.spawn(async move {
                            match client
                                .database(&db)
                                .collection::<Document>(&collection)
                                .find(None, None)
                                .await
                            {
                                Ok(collection) => match collection.try_collect().await {
                                    Err(ex) => sender.send(Response::MongoError(ex)),
                                    Ok(val) => sender.send(Response::DownloadedDocuments(val)),
                                },
                                Err(ex) => sender.send(Response::MongoError(ex)),
                            }
                                .expect("Sender for `ClusterClient::download_collection` dropped");
                        });
                    }
                }
            }
            info!("Thing yeet'd");
        });

        Ok(Self { tx })
    }

    /// Gets all the databases and collections in the cluster
    pub async fn get_collections(&self) -> MongoResult<Vec<(String, Vec<String>)>> {
        let (tx, rx) = oneshot::channel();
        self.tx.send((Request::ListCollections, tx)).await.unwrap();

        match rx
            .await
            .expect("Sender for `ClusterClient::get_collections` dropped")
        {
            Response::MongoError(err) => Err(err),
            Response::Collections(ok) => Ok(ok),
            _ => unreachable!(),
        }
    }

    /// Download a collection from cluster
    pub async fn download_collection(
        &self,
        db: String,
        collection: String,
    ) -> MongoResult<Vec<Document>> {
        let (tx, rx) = oneshot::channel();
        self.tx
            .send((Request::DownloadCollection { collection, db }, tx))
            .await
            .unwrap();

        match rx
            .await
            .expect("Sender for `ClusterClient::get_collections` dropped")
        {
            Response::MongoError(err) => Err(err),
            Response::DownloadedDocuments(ok) => Ok(ok),
            _ => unreachable!(),
        }
    }

    /// Upload a collection to cluster
    pub async fn upload_collection(
        &self,
        db: String,
        collection: String,
        documents: Vec<Document>,
    ) -> MongoResult<()> {
        let (tx, rx) = oneshot::channel();
        self.tx
            .send((
                Request::UploadCollection {
                    collection,
                    db,
                    documents,
                },
                tx,
            ))
            .await
            .unwrap();

        match rx
            .await
            .expect("Sender for `ClusterClient::get_collections` dropped")
        {
            Response::MongoError(err) => Err(err),
            Response::UploadSuccess => Ok(()),
            _ => unreachable!(),
        }
    }
}
