use crate::models::{ApiRequest, Collection, Environment};
use sled::{Db, Tree};
use std::error::Error;

pub struct StorageManager {
    _db: Db, // Keep the root db alive
    requests_tree: Tree,
    collections_tree: Tree,
    environments_tree: Tree,
}

impl StorageManager {
    /// Opens or creates a new sled database at the given path
    pub fn new(path: &str) -> Result<Self, sled::Error> {
        let db = sled::open(path)?;

        // Create isolated namespaces (trees) for different data types
        let requests_tree = db.open_tree(b"requests")?;
        let collections_tree = db.open_tree(b"collections")?;
        let environments_tree = db.open_tree(b"environments")?;

        Ok(Self {
            _db: db,
            requests_tree,
            collections_tree,
            environments_tree,
        })
    }

    // --- ENVIRONMENT STORAGE ---

    pub fn save_environment(&self, env: &Environment) -> Result<(), Box<dyn Error>> {
        let value = serde_json::to_vec(env)?;
        self.environments_tree.insert(env.id.as_bytes(), value)?;
        self.environments_tree.flush()?;

        Ok(())
    }

    pub fn get_environment(&self, id: &str) -> Result<Option<Environment>, Box<dyn Error>> {
        if let Some(bytes) = self.environments_tree.get(id.as_bytes())? {
            Ok(Some(serde_json::from_slice(&bytes)?))
        } else {
            Ok(None)
        }
    }

    // --- COLLECTION STORAGE ---

    pub fn save_collection(&self, collection: &Collection) -> Result<(), Box<dyn Error>> {
        let value = serde_json::to_vec(collection)?;
        self.collections_tree
            .insert(collection.id.as_bytes(), value)?;
        self.collections_tree.flush()?;

        Ok(())
    }

    pub fn get_collection(&self, id: &str) -> Result<Option<Collection>, Box<dyn Error>> {
        if let Some(bytes) = self.collections_tree.get(id.as_bytes())? {
            Ok(Some(serde_json::from_slice(&bytes)?))
        } else {
            Ok(None)
        }
    }

    /// Serializes an ApiRequest to JSON and saves it to sled
    pub fn save_request(&self, request: &ApiRequest) -> Result<(), Box<dyn Error>> {
        // We use the request's unique ID as the key
        let key = request.id.as_bytes();

        // Convert the Rust struct into a JSON byte vector
        let value = serde_json::to_vec(request)?;

        // Insert the key-value pair into the database
        self.requests_tree.insert(key, value)?;

        // Ensure the data is flushed to disk safely
        self.requests_tree.flush()?;

        Ok(())
    }

    /// Retrieves an ApiRequest by its ID and deserializes it back into a Rust struct
    pub fn get_request(&self, id: &str) -> Result<Option<ApiRequest>, Box<dyn Error>> {
        let result = self.requests_tree.get(id.as_bytes())?;

        match result {
            Some(bytes) => {
                // Convert the bytes back into our ApiRequest struct
                let request: ApiRequest = serde_json::from_slice(&bytes)?;
                Ok(Some(request))
            }
            None => Ok(None),
        }
    }

    /// Fetches all saved requests (great for a sidebar history view)
    pub fn get_all_requests(&self) -> Result<Vec<ApiRequest>, Box<dyn Error>> {
        let mut requests = Vec::new();

        // Iterate through all key-value pairs in the database
        for item in self.requests_tree.iter() {
            let (_, value) = item?;
            let request: ApiRequest = serde_json::from_slice(&value)?;
            requests.push(request);
        }
        Ok(requests)
    }
}
