/// A resource manager for Mutex and Semaphore.

use crate::sync::resource_check::ResourceCheck;

struct ResourceManager {
    mutex_manager: ResourceCheck,
    semaphore_manager: ResourceCheck,
}