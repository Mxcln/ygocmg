export interface CardAssetStateDto {
  has_image: boolean;
  has_field_image: boolean;
  has_script: boolean;
}

export interface ImportMainImageInput {
  workspaceId: string;
  packId: string;
  cardId: string;
  sourcePath: string;
}

export interface DeleteMainImageInput {
  workspaceId: string;
  packId: string;
  cardId: string;
}

export interface ImportFieldImageInput {
  workspaceId: string;
  packId: string;
  cardId: string;
  sourcePath: string;
}

export interface DeleteFieldImageInput {
  workspaceId: string;
  packId: string;
  cardId: string;
}

export interface CreateEmptyScriptInput {
  workspaceId: string;
  packId: string;
  cardId: string;
}

export interface ImportScriptInput {
  workspaceId: string;
  packId: string;
  cardId: string;
  sourcePath: string;
}

export interface DeleteScriptInput {
  workspaceId: string;
  packId: string;
  cardId: string;
}

export interface OpenScriptExternalInput {
  workspaceId: string;
  packId: string;
  cardId: string;
}
