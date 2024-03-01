from datetime import datetime
from typing import TYPE_CHECKING
from uuid import uuid4

from sqlalchemy import DateTime, ForeignKey, func
from sqlalchemy.orm import Mapped, mapped_column

from bootstrap.db import Base

if TYPE_CHECKING:
    from .user import User


class Heartbeat(Base):
    __tablename__ = "heartbeats"
    id: Mapped[str] = mapped_column(primary_key=True, index=True, default=uuid4().hex)
    user_id: Mapped[str] = mapped_column(ForeignKey(User.id))
    entity: Mapped[str] = mapped_column(index=True)
    type: Mapped[str] = mapped_column(index=True)
    category: Mapped[str] = mapped_column(index=True)
    browser: Mapped[str] = mapped_column(index=True)

    domain: Mapped[str] = mapped_column(index=True)  # page domain
    path: Mapped[str] = mapped_column(index=True)  # page path
    user_agent: Mapped[str] = mapped_column()  # user agent

    time: Mapped[datetime] = mapped_column(
        DateTime(timezone=True), index=True, nullable=True
    )
    hash: Mapped[str] = mapped_column(index=True, unique=True)

    created_at: Mapped[datetime] = mapped_column(
        DateTime(timezone=True),
        server_default=func.now(),
    )
