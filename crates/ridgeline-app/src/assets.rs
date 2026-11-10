use std::borrow::Cow;

use gpui_kit::{AssetSource, Result, SharedString};

// gpui-kit only embeds its default icon set; every other Lucide icon the UI
// renders has to be listed here or it silently draws nothing.
gpui_kit::assets::icon_assets!(
    ExtraIcons,
    [
        CircleCheck,
        Clock,
        FilePen,
        GitBranch,
        GitMerge,
        Hourglass,
        Loader,
        MessageSquare,
        Minus,
        TriangleAlert,
        UserX,
        Users,
        X
    ]
);

pub struct AppAssets;

impl AssetSource for AppAssets {
    fn load(&self, path: &str) -> Result<Option<Cow<'static, [u8]>>> {
        if let Some(bytes) = ExtraIcons.load(path)? {
            return Ok(Some(bytes));
        }
        gpui_kit::assets::Assets.load(path)
    }

    fn list(&self, path: &str) -> Result<Vec<SharedString>> {
        let mut paths = gpui_kit::assets::Assets.list(path)?;
        paths.extend(ExtraIcons.list(path)?);
        paths.sort();
        paths.dedup();
        Ok(paths)
    }
}
