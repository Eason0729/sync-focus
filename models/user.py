from datetime import datetime
from uuid import UUID, uuid4

from sqlalchemy import DateTime, func
from sqlalchemy.orm import Mapped, mapped_column

from .base import Base


# Create User class
class User(Base):
    __tablename__ = "users"
    id: Mapped[UUID] = mapped_column(primary_key=True, index=True, default=uuid4)
    username: Mapped[str] = mapped_column(unique=True, index=True)
    password: Mapped[str] = mapped_column()
    api_key: Mapped[str] = mapped_column(unique=True, default=uuid4, index=True)
    is_admin: Mapped[bool] = mapped_column(default=False)
    last_login: Mapped[datetime] = mapped_column(
        DateTime(timezone=True), server_default=func.now()
    )
