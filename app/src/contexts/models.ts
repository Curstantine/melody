import { createContext, useContext } from "solid-js";

import type AppModel from "@/models/App";
import type SetupViewModel from "@/pages/setup/model";

export const AppModelContext = createContext<AppModel>(undefined, { name: "AppModelContext" });
export const useAppModel = () => useContext(AppModelContext)!;

export const SetupViewModelContext = createContext<SetupViewModel>(undefined, { name: "SetupViewModelContext" });
export const useSetupViewModel = () => useContext(SetupViewModelContext)!;
