use std::any::TypeId;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use crate::component::Interface;
use crate::container::{Map, RegisteredType};
use crate::Container;
use crate::Error as DIError;
use crate::Result as DIResult;

/// Holds the registration and resolved components while building a [Container]. This struct is
/// used during [Component::build].
///
/// [Container]: struct.Container.html
/// [Component::build]: component/trait.Component.html#tymethod.build
pub struct ContainerBuildContext {
    registration_map: HashMap<TypeId, RegisteredType>,
    resolved_map: Map,
}

impl ContainerBuildContext {
    pub(crate) fn new(registration_map: HashMap<TypeId, RegisteredType>) -> Self {
        ContainerBuildContext {
            registration_map,
            resolved_map: Map::new(),
        }
    }

    pub(crate) fn build(mut self) -> DIResult<Container> {
        // Order the registrations so dependencies are resolved first (topological sort)
        let sorted_registrations = self.sort_registrations_by_dependencies()?;

        for mut registration in sorted_registrations {
            // Each component will add itself into resolved_map via insert_resolved_component
            registration.build(&mut self)?;
        }

        Ok(Container::new(self.resolved_map))
    }

    fn sort_registrations_by_dependencies(&mut self) -> DIResult<Vec<RegisteredType>> {
        let mut visited = HashSet::new();
        let mut sorted = Vec::new();

        while let Some(interface_id) = self.registration_map.keys().next().copied() {
            let registration = self.registration_map.remove(&interface_id).unwrap();

            if !visited.contains(&interface_id) {
                self.registration_sort_visit(registration, &mut visited, &mut sorted)?;
            }
        }

        Ok(sorted)
    }

    fn registration_sort_visit(
        &mut self,
        registration: RegisteredType,
        visited: &mut HashSet<TypeId>,
        sorted: &mut Vec<RegisteredType>,
    ) -> DIResult<()> {
        visited.insert(registration.interface_id);

        for dependency in &registration.dependencies {
            if !visited.contains(&dependency.type_id) {
                let dependency_registration = self
                    .registration_map
                    .remove(&dependency.type_id)
                    .ok_or_else(|| {
                        DIError::ResolveError(format!(
                            "Unable to resolve dependency '{}: {}' of component '{}'",
                            dependency.name, dependency.type_name, registration.component
                        ))
                    })?;

                self.registration_sort_visit(dependency_registration, visited, sorted)?;
            }
        }

        sorted.push(registration);
        Ok(())
    }

    pub fn resolve_component<I: Interface + ?Sized>(&mut self) -> DIResult<Arc<I>> {
        self.resolved_map
            .get::<Arc<I>>()
            .map(Arc::clone)
            .ok_or_else(|| {
                DIError::ResolveError(format!(
                    "Component {} has not yet been resolved, or is not registered. Check your dependencies.",
                    ::std::any::type_name::<I>()
                ))
            })
    }

    pub fn insert_resolved_component<I: Interface + ?Sized>(&mut self, component: Box<I>) {
        self.resolved_map.insert::<Arc<I>>(Arc::from(component));
    }
}