import AppModel from "@/models/App";
import { createContext, useContext } from "solid-js";

export const AppModelContext = createContext<AppModel>(undefined, { name: "AppModelContext" });
export const useAppModelContext = () => useContext(AppModelContext)!;
