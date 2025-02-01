## Lessons Learned
- `.await` can only be used on types that implement Future
- `?` operator requires the type to implement Try trait
- Arc provides immutable shared ownership - use Arc::make_mut for mutable access
- RwLockWriteGuard needs to be used carefully to avoid shadowing issues
- Struct definitions must be unique within a module's namespace
- Test modules need explicit imports for all types used
- Derived traits must be implemented for all fields of a struct
