// export
export type ResourceType = "image";
export type ResourceRelationType = "artist" | "release" | "track";
export type ResourceMediaType = "png" | "jpeg";

export interface Resource {
	type: ResourceType;
	relation_type: ResourceRelationType;
	media_type: ResourceMediaType;
	hash: string;
}

export type DisplayImageResource = Resource & {
	source_path: string;
	thumb_path?: string;
};
