export type ResourceType = "artist" | "release" | "track";
export type ResourceMediaType = "png" | "jpeg";

export interface Resource {
	type: ResourceType;
	media_type: ResourceMediaType;
	path: string;
	hash: string;
}
