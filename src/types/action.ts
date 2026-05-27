export type PlanStatus = "ready" | "needsUser" | "blocked";

export type GhostPreview = {
  cursorPath?: Array<{ x: number; y: number }>;
  highlight?: { x: number; y: number; width: number; height: number } | null;
  narration?: string | null;
};

export type ExpectedState = {
  kind: string;
  value: string;
};

export type SkillAction =
  | {
      action: "photoshop.crop";
      params: { aspect?: string; bounds?: { x: number; y: number; width: number; height: number } };
    }
  | {
      action: "photoshop.adjust.levels";
      params: { inputBlack?: number; inputWhite?: number; gamma?: number };
    }
  | {
      action: "photoshop.export.png";
      params: { path?: string; quality?: number };
    }
  | {
      action: "photoshop.export.jpeg";
      params: { path?: string; quality?: number };
    }
  | {
      action: "photoshop.resize.canvas";
      params: { width: number; height: number; unit: "px" | "percent" };
    }
  | {
      action: "photoshop.rotate";
      params: { angle: number };
    }
  | {
      action: "photoshop.straighten";
      params: { angle: number };
    }
  | {
      action: "desktop.open_app";
      params: { path: string; args?: string[] };
    }
  | {
      action: "desktop.check_app";
      params: { appName: string };
    }
  | {
      action: "desktop.click";
      params: { x: number; y: number; button?: string; clickCount?: number };
    }
  | {
      action: "desktop.move_mouse";
      params: { x: number; y: number };
    }
  | {
      action: "desktop.type_text";
      params: { text: string };
    }
  | {
      action: "desktop.hotkey";
      params: { keys: string[] };
    }
  | {
      action: "desktop.scroll";
      params: { amount: number };
    }
  | {
      action: "desktop.wait";
      params: { ms: number };
    }
  | {
      action: "desktop.ui_click";
      params: { name: string; controlType?: string; windowName?: string };
    }
  | {
      action: "desktop.ui_type";
      params: { name: string; text: string; controlType?: string; windowName?: string };
    }
  | {
      action: "desktop.ui_read";
      params: { name: string; controlType?: string; windowName?: string };
    };

export type ActionStep = {
  stepId: string;
  requiresConfirm: boolean;
  action: SkillAction;
  expectedResult?: string | null;
  retryable: boolean;
  explanation?: string | null;
  expected: ExpectedState[];
  ghostPreview: GhostPreview;
  note?: string | null;
};

export type ActionPlan = {
  planId: string;
  createdAt: string;
  goal?: string | null;
  status: PlanStatus;
  steps: ActionStep[];
  note?: string | null;
};
