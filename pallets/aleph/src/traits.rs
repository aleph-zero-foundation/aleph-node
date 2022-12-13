use primitives::SessionIndex;

/// Information provider from `pallet_session`. Loose pallet coupling via traits.
pub trait SessionInfoProvider<T: frame_system::Config> {
    fn current_session() -> SessionIndex;
}

impl<T> SessionInfoProvider<T> for pallet_session::Pallet<T>
where
    T: pallet_session::Config,
{
    fn current_session() -> SessionIndex {
        pallet_session::CurrentIndex::<T>::get()
    }
}
