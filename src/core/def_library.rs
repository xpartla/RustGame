// Generic definition-library machinery — the single implementation behind AbilityLibrary,
// TalentLibrary, StatusLibrary, and HeroLibrary.
//
// Before Phase 4 these were three hand-copied triples (a `XLibrary` resource + a `XDefLoader`
// AssetLoader + a `load_x_defs` Startup system) that differed only in the asset type, the file
// extension, and the id → path manifest. Adding HeroDef would have made a fourth copy — so §8.5
// of the architecture plan required generalizing them "at Phase 4 start, before HeroDef adds a
// fourth copy." This module is that generic:
//
//   - `DefLibrary<T>`   — the id → Handle<T> registry resource (replaces every `XLibrary`).
//   - `DefAsset`        — per-def-type metadata: the RON extension(s) and the load manifest.
//   - `RonDefLoader<T>` — one AssetLoader that ron-deserializes any `DefAsset` (replaces every `XDefLoader`).
//   - `register_def_library::<T>()` — an App extension that wires asset + loader + resource +
//                         the Startup populate system in one call (replaces every `load_x_defs`).
//
// Each concrete def keeps its public name via a type alias, e.g.
//   `pub type AbilityLibrary = DefLibrary<AbilityDef>;`
// so no downstream `Res<AbilityLibrary>` / `library.get(id)` / `library.defs` call site changes.

use bevy::asset::io::Reader;
use bevy::asset::{AssetApp, AssetLoader, LoadContext};
use bevy::prelude::*;
use std::collections::HashMap;
use std::marker::PhantomData;

/// Per-def-type metadata that lets one generic loader + one registration helper serve every
/// RON-backed definition asset.
pub trait DefAsset: Asset + for<'de> serde::Deserialize<'de> {
    /// File extension(s) this def's RON files use, e.g. `&["ability.ron"]`. Must be distinct per
    /// def type so the loaders never collide on a shared plain `.ron`.
    const EXTENSIONS: &'static [&'static str];
    /// The (id, asset_path) list loaded into the library at startup. A fixed manifest for now;
    /// a later phase can replace this with a folder scan.
    const MANIFEST: &'static [(&'static str, &'static str)];
}

/// Maps a stable string id to the handle of its loaded definition asset. One instance per def
/// type via a type alias (AbilityLibrary, TalentLibrary, StatusLibrary, HeroLibrary).
#[derive(Resource)]
pub struct DefLibrary<T: Asset> {
    pub defs: HashMap<String, Handle<T>>,
}

// Hand-written (not derived) so the bound is `T: Asset`, not `T: Default`.
impl<T: Asset> Default for DefLibrary<T> {
    fn default() -> Self {
        Self { defs: HashMap::new() }
    }
}

impl<T: Asset> DefLibrary<T> {
    /// Resolves an id to its asset handle, if the id is in the manifest.
    pub fn get(&self, id: &str) -> Option<&Handle<T>> {
        self.defs.get(id)
    }
}

/// A single AssetLoader that ron-deserializes any `DefAsset`. Replaces the per-type
/// `AbilityDefLoader` / `TalentDefLoader` / `StatusEffectDefLoader`, which were byte-identical
/// apart from the asset type and extension.
pub struct RonDefLoader<T: DefAsset>(PhantomData<fn() -> T>);

impl<T: DefAsset> Default for RonDefLoader<T> {
    fn default() -> Self {
        Self(PhantomData)
    }
}

impl<T: DefAsset> AssetLoader for RonDefLoader<T> {
    type Asset = T;
    type Settings = ();
    type Error = Box<dyn std::error::Error + Send + Sync>;

    async fn load(
        &self,
        reader: &mut dyn Reader,
        _settings: &(),
        _load_context: &mut LoadContext<'_>,
    ) -> Result<T, Self::Error> {
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).await?;
        let def = ron::de::from_bytes::<T>(&bytes)?;
        Ok(def)
    }

    fn extensions(&self) -> &[&str] {
        T::EXTENSIONS
    }
}

/// Populates a `DefLibrary<T>` from `T::MANIFEST` at startup. Generic over the def type — one
/// system definition serves every library.
fn populate_def_library<T: DefAsset>(
    asset_server: Res<AssetServer>,
    mut library: ResMut<DefLibrary<T>>,
) {
    for (id, path) in T::MANIFEST {
        library.defs.insert((*id).to_string(), asset_server.load(*path));
    }
}

/// App extension: register a def type's asset, RON loader, library resource, and Startup
/// populate system in one call. Replaces the four-line boilerplate each plugin used to repeat.
pub trait DefLibraryAppExt {
    fn register_def_library<T: DefAsset>(&mut self) -> &mut Self;
}

impl DefLibraryAppExt for App {
    fn register_def_library<T: DefAsset>(&mut self) -> &mut Self {
        self.init_asset::<T>()
            .register_asset_loader(RonDefLoader::<T>::default())
            .init_resource::<DefLibrary<T>>()
            .add_systems(Startup, populate_def_library::<T>);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Minimal Asset stand-in so the registry can be exercised without an App or loaded assets.
    #[derive(Asset, TypePath)]
    struct DummyDef;

    #[test]
    fn get_returns_handle_for_known_id_and_none_for_unknown() {
        let mut lib: DefLibrary<DummyDef> = DefLibrary::default();
        let handle: Handle<DummyDef> = Handle::default();
        lib.defs.insert("known".to_string(), handle.clone());
        assert_eq!(lib.get("known"), Some(&handle));
        assert!(lib.get("missing").is_none());
    }
}
