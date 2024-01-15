// export
export type ResourceType = "image";
export type ResourceRelationType = "artist" | "release" | "track";
export type ResourceMediaType = "png" | "jpeg";

export interface Cover {
	type: ResourceType;
	relation_type: ResourceRelationType;
	media_type: ResourceMediaType;
	hash: string;
}

export type DisplayCover = Cover & { path: string };
