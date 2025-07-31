from functools import lru_cache
from pydantic import BaseSettings, Field, AnyUrl


class Settings(BaseSettings):
    """Application settings loaded from environment variables or .env file."""

    # General
    ENV: str = Field("development", env="ENV")
    DEBUG: bool = Field(True, env="DEBUG")
    APP_NAME: str = "EmailBrain"

    # API
    API_V1_PREFIX: str = "/api/v1"

    # CORS
    FRONTEND_ORIGIN: str = Field("http://localhost:3000", env="FRONTEND_ORIGIN")

    # Database
    DATABASE_URL: str = Field("sqlite+aiosqlite:///./backend/db/mail.db", env="DATABASE_URL")

    # LM Studio
    LM_STUDIO_URL: AnyUrl = Field("http://127.0.0.1:1234", env="LM_STUDIO_URL")
    LM_TIMEOUT_SECONDS: int = Field(30, env="LM_TIMEOUT_SECONDS")

    class Config:
        case_sensitive = True
        env_file = ".env"
        env_file_encoding = "utf-8"


@lru_cache()
def get_settings() -> Settings:  # pragma: no cover
    return Settings()


settings = get_settings()
