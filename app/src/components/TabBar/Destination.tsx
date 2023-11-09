import { Link } from "@solidjs/router";

export type Props = {
    href: string;
    label: string;
}

export default function TabBarDestination(props: Props) {
    return (
        <Link href={props.href} class="text-text-2 use-transition-standard hover:text-text-1" activeClass="text-text-1">
            {props.label}
        </Link>
    )
}