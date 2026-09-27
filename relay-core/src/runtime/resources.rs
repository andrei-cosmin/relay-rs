use std::{
    any::{Any, TypeId, type_name},
    collections::HashMap,
    sync::Arc,
};

pub struct Resources {
    items: HashMap<TypeId, Arc<dyn Any + Send + Sync>>,
}

impl Resources {
    pub fn new() -> Self {
        Self { items: HashMap::new() }
    }

    pub fn insert<T: Send + Sync + 'static>(&mut self, value: T) {
        self.items.insert(TypeId::of::<T>(), Arc::new(value));
    }

    pub fn get<T: Send + Sync + 'static>(&self) -> Arc<T> {
        let Some(item) = self.items.get(&TypeId::of::<T>()) else {
            panic!("{} was asked for before it was registered", type_name::<T>());
        };
        item.clone().downcast::<T>().expect("resource type matches its key")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Port(u16);

    #[test]
    fn get_returns_the_inserted_value_shared() {
        let mut resources = Resources::new();
        resources.insert(Port(8585));
        assert_eq!(resources.get::<Port>().0, 8585);
        assert!(Arc::ptr_eq(&resources.get::<Port>(), &resources.get::<Port>()));
    }

    #[test]
    fn inserting_the_same_type_again_replaces_it() {
        let mut resources = Resources::new();
        resources.insert(Port(1));
        resources.insert(Port(2));
        assert_eq!(resources.get::<Port>().0, 2);
    }

    #[test]
    #[should_panic(expected = "Port was asked for before it was registered")]
    fn a_missing_resource_is_named_in_the_panic() {
        Resources::new().get::<Port>();
    }
}
