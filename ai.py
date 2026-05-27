import os
import sys

import google.generativeai as genai


def load_env_file(file_path: str) -> dict[str, str]:
    if not os.path.exists(file_path):
        return {}

    data: dict[str, str] = {}
    with open(file_path, "r", encoding="utf-8") as handle:
        for line in handle:
            stripped = line.strip()
            if not stripped or stripped.startswith("#"):
                continue
            if "=" not in stripped:
                continue
            key, value = stripped.split("=", 1)
            key = key.strip()
            value = value.strip()
            if value.startswith(("\"", "'")) and value.endswith(("\"", "'")):
                value = value[1:-1]
            data[key] = value
    return data


def get_env_value(env_file: dict[str, str], name: str, fallback: str) -> str:
    if name in os.environ:
        return os.environ[name]
    if name in env_file:
        return env_file[name]
    return fallback


def parse_keys(raw: str) -> list[str]:
    return [value.strip() for value in raw.split(",") if value.strip()]


def mask_key(value: str) -> str:
    if len(value) < 10:
        return "REDACTED"
    return f"{value[:4]}...{value[-4:]}"


def load_max_output_tokens(env_file: dict[str, str]) -> int:
    raw = get_env_value(env_file, "GEMINI_MAX_OUTPUT_TOKENS", "160")
    try:
        parsed = int(raw)
    except ValueError:
        return 160
    return parsed if parsed > 0 else 160


def main() -> int:
    env_file = load_env_file(".env")
    raw_keys = get_env_value(env_file, "GEMINI_API_KEYS", "")
    keys = parse_keys(raw_keys)
    if not keys:
        print("Missing GEMINI_API_KEYS. Set it in your environment before running this script.")
        return 1

    model_name = get_env_value(env_file, "GEMINI_MODEL", "gemini-2.5-flash-lite")
    prompt = get_env_value(env_file, "GEMINI_TEST_PROMPT", "ping")
    max_output_tokens = load_max_output_tokens(env_file)
    
    for index, key in enumerate(keys, start=1):
        genai.configure(api_key=key)
        model = genai.GenerativeModel(model_name)
        try:
            response = model.generate_content(
                prompt,
                generation_config=genai.types.GenerationConfig(
                    temperature=0.1,
                    max_output_tokens=max_output_tokens
                )
            )
        except Exception as error:
            print(f"Key {index} ({mask_key(key)}): FAIL - {error}")
            continue

        text = getattr(response, "text", "")
        if text:
            print(f"Key {index} ({mask_key(key)}): OK")
            print(text)
            return 0

        print(f"Key {index} ({mask_key(key)}): FAIL - Empty response")

    return 1


if __name__ == "__main__":
    sys.exit(main())