type Props = {
	// eslint-disable-next-line @typescript-eslint/no-explicit-any
	error: any;
	reset: () => void;
};

export default function AppError({ error }: Props) {
	return (
		<div class="flex h-full w-full ">
			{error}
		</div>
	);
}
