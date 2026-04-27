use std::process::Command;

use crate::application::dto::common::WriteResultDto;
use crate::application::dto::resource::{
    CardAssetStateDto, CreateEmptyScriptInput, DeleteFieldImageInput, DeleteMainImageInput,
    DeleteScriptInput, ImportFieldImageInput, ImportMainImageInput, ImportScriptInput,
    OpenScriptExternalInput,
};
use crate::bootstrap::AppState;
use crate::domain::common::error::{AppError, AppResult};

pub struct ResourceService<'a> {
    state: &'a AppState,
}

impl<'a> ResourceService<'a> {
    pub fn new(state: &'a AppState) -> Self {
        Self { state }
    }

    pub fn import_main_image(
        &self,
        input: ImportMainImageInput,
    ) -> AppResult<WriteResultDto<CardAssetStateDto>> {
        let state = crate::application::pack::write_service::PackWriteService::new(self.state)
            .import_main_image(&input.workspace_id, &input.pack_id, &input.card_id, &input.source_path)?;
        Ok(ok_state(state))
    }

    pub fn delete_main_image(
        &self,
        input: DeleteMainImageInput,
    ) -> AppResult<WriteResultDto<CardAssetStateDto>> {
        let state = crate::application::pack::write_service::PackWriteService::new(self.state)
            .delete_main_image(&input.workspace_id, &input.pack_id, &input.card_id)?;
        Ok(ok_state(state))
    }

    pub fn import_field_image(
        &self,
        input: ImportFieldImageInput,
    ) -> AppResult<WriteResultDto<CardAssetStateDto>> {
        let state = crate::application::pack::write_service::PackWriteService::new(self.state)
            .import_field_image(&input.workspace_id, &input.pack_id, &input.card_id, &input.source_path)?;
        Ok(ok_state(state))
    }

    pub fn delete_field_image(
        &self,
        input: DeleteFieldImageInput,
    ) -> AppResult<WriteResultDto<CardAssetStateDto>> {
        let state = crate::application::pack::write_service::PackWriteService::new(self.state)
            .delete_field_image(&input.workspace_id, &input.pack_id, &input.card_id)?;
        Ok(ok_state(state))
    }

    pub fn create_empty_script(
        &self,
        input: CreateEmptyScriptInput,
    ) -> AppResult<WriteResultDto<CardAssetStateDto>> {
        let state = crate::application::pack::write_service::PackWriteService::new(self.state)
            .create_empty_script(&input.workspace_id, &input.pack_id, &input.card_id)?;
        Ok(ok_state(state))
    }

    pub fn import_script(
        &self,
        input: ImportScriptInput,
    ) -> AppResult<WriteResultDto<CardAssetStateDto>> {
        let state = crate::application::pack::write_service::PackWriteService::new(self.state)
            .import_script(&input.workspace_id, &input.pack_id, &input.card_id, &input.source_path)?;
        Ok(ok_state(state))
    }

    pub fn delete_script(
        &self,
        input: DeleteScriptInput,
    ) -> AppResult<WriteResultDto<CardAssetStateDto>> {
        let state = crate::application::pack::write_service::PackWriteService::new(self.state)
            .delete_script(&input.workspace_id, &input.pack_id, &input.card_id)?;
        Ok(ok_state(state))
    }

    pub fn open_script_external(&self, input: OpenScriptExternalInput) -> AppResult<()> {
        crate::application::pack::service::ensure_workspace_matches(self.state, &input.workspace_id)?;
        let snapshot = crate::application::pack::service::require_open_pack_snapshot(
            self.state,
            &input.workspace_id,
            &input.pack_id,
        )?;
        let card = snapshot
            .cards
            .iter()
            .find(|card| card.id == input.card_id)
            .ok_or_else(|| AppError::new("card.not_found", "card was not found"))?;

        let config = crate::application::config::service::ConfigService::new(self.state).load()?;
        let editor_path = config.external_text_editor_path.ok_or_else(|| {
            AppError::new(
                "resource.external_editor_not_configured",
                "external text editor path is not configured",
            )
        })?;
        if !editor_path.exists() {
            return Err(
                AppError::new(
                    "resource.external_editor_missing",
                    "external text editor executable does not exist",
                )
                .with_detail("path", editor_path.display().to_string()),
            );
        }

        let script_path = crate::domain::resource::path_rules::script_path(&snapshot.pack_path, card.code);
        if !script_path.exists() {
            return Err(
                AppError::new("resource.script_missing", "script file does not exist")
                    .with_detail("path", script_path.display().to_string()),
            );
        }

        Command::new(&editor_path)
            .arg(&script_path)
            .spawn()
            .map_err(|source| {
                AppError::from_io("resource.external_editor_launch_failed", source)
                    .with_detail("editor_path", editor_path.display().to_string())
                    .with_detail("script_path", script_path.display().to_string())
            })?;
        Ok(())
    }
}

fn ok_state(state: crate::domain::resource::model::CardAssetState) -> WriteResultDto<CardAssetStateDto> {
    WriteResultDto::Ok {
        data: state.into(),
        warnings: Vec::new(),
    }
}
