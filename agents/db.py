"""async database connection management.

provides a connection pool via sqlalchemy async engine.
database url comes from the DATABASE_URL environment variable.
"""

from __future__ import annotations

import os

from sqlalchemy.ext.asyncio import AsyncEngine, create_async_engine


_engine: AsyncEngine | None = None


def get_database_url() -> str:
    """get database url from environment. falls back to local postgres."""
    return os.environ.get(
        "DATABASE_URL",
        "postgresql+asyncpg://postgres:postgres@localhost:5432/trading",
    )


async def get_engine() -> AsyncEngine:
    """get or create the async database engine."""
    global _engine
    if _engine is None:
        _engine = create_async_engine(
            get_database_url(),
            pool_size=5,
            max_overflow=2,
            echo=False,
        )
    return _engine


async def close_engine() -> None:
    """dispose of the database engine."""
    global _engine
    if _engine is not None:
        await _engine.dispose()
        _engine = None
