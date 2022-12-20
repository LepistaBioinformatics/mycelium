-- CreateTable
CREATE TABLE "user" (
    "id" TEXT NOT NULL,
    "username" VARCHAR(140) NOT NULL,
    "email" VARCHAR(140) NOT NULL,
    "first_name" VARCHAR(140) NOT NULL,
    "last_name" VARCHAR(140) NOT NULL,
    "is_active" BOOLEAN NOT NULL DEFAULT true,
    "created" TIMESTAMPTZ(6) NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updated" TIMESTAMPTZ(6),

    CONSTRAINT "user_pkey" PRIMARY KEY ("id")
);

-- CreateTable
CREATE TABLE "role" (
    "id" TEXT NOT NULL,
    "name" VARCHAR(140) NOT NULL,
    "description" VARCHAR(255) NOT NULL,

    CONSTRAINT "role_pkey" PRIMARY KEY ("id")
);

-- CreateTable
CREATE TABLE "guest_role" (
    "id" TEXT NOT NULL,
    "name" VARCHAR(140) NOT NULL,
    "description" VARCHAR(255),
    "role_id" TEXT NOT NULL,
    "permissions" INTEGER[],

    CONSTRAINT "guest_role_pkey" PRIMARY KEY ("id")
);

-- CreateTable
CREATE TABLE "guest_user" (
    "id" TEXT NOT NULL,
    "email" TEXT NOT NULL,
    "role_id" TEXT NOT NULL,
    "created" TIMESTAMPTZ(6) NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updated" TIMESTAMPTZ(6),

    CONSTRAINT "guest_user_pkey" PRIMARY KEY ("id")
);

-- CreateTable
CREATE TABLE "account_type" (
    "id" TEXT NOT NULL,
    "name" VARCHAR(128) NOT NULL,
    "description" VARCHAR(255) NOT NULL,
    "is_subscription" BOOLEAN NOT NULL DEFAULT false,
    "is_manager" BOOLEAN NOT NULL DEFAULT false,
    "is_staff" BOOLEAN NOT NULL DEFAULT false,

    CONSTRAINT "account_type_pkey" PRIMARY KEY ("id")
);

-- CreateTable
CREATE TABLE "account" (
    "id" TEXT NOT NULL,
    "name" VARCHAR(64) NOT NULL,
    "owner_id" TEXT NOT NULL,
    "account_type_id" TEXT NOT NULL,
    "created" TIMESTAMPTZ(6) NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updated" TIMESTAMPTZ(6),
    "is_active" BOOLEAN NOT NULL DEFAULT true,
    "is_checked" BOOLEAN NOT NULL DEFAULT false,

    CONSTRAINT "account_pkey" PRIMARY KEY ("id")
);

-- CreateTable
CREATE TABLE "guest_user_on_account" (
    "guest_user_id" TEXT NOT NULL,
    "account_id" TEXT NOT NULL,
    "created" TIMESTAMPTZ(6) NOT NULL DEFAULT CURRENT_TIMESTAMP,

    CONSTRAINT "guest_user_on_account_pkey" PRIMARY KEY ("guest_user_id","account_id")
);

-- CreateIndex
CREATE UNIQUE INDEX "user_email_key" ON "user"("email");

-- CreateIndex
CREATE UNIQUE INDEX "guest_user_email_role_id_key" ON "guest_user"("email", "role_id");

-- CreateIndex
CREATE UNIQUE INDEX "account_type_name_key" ON "account_type"("name");

-- CreateIndex
CREATE UNIQUE INDEX "account_owner_id_key" ON "account"("owner_id");

-- CreateIndex
CREATE UNIQUE INDEX "account_name_owner_id_key" ON "account"("name", "owner_id");

-- CreateIndex
CREATE UNIQUE INDEX "guest_user_on_account_guest_user_id_account_id_key" ON "guest_user_on_account"("guest_user_id", "account_id");

-- AddForeignKey
ALTER TABLE "guest_role" ADD CONSTRAINT "guest_role_role_id_fkey" FOREIGN KEY ("role_id") REFERENCES "role"("id") ON DELETE RESTRICT ON UPDATE CASCADE;

-- AddForeignKey
ALTER TABLE "guest_user" ADD CONSTRAINT "guest_user_role_id_fkey" FOREIGN KEY ("role_id") REFERENCES "guest_role"("id") ON DELETE RESTRICT ON UPDATE CASCADE;

-- AddForeignKey
ALTER TABLE "account" ADD CONSTRAINT "account_owner_id_fkey" FOREIGN KEY ("owner_id") REFERENCES "user"("id") ON DELETE RESTRICT ON UPDATE CASCADE;

-- AddForeignKey
ALTER TABLE "account" ADD CONSTRAINT "account_account_type_id_fkey" FOREIGN KEY ("account_type_id") REFERENCES "account_type"("id") ON DELETE RESTRICT ON UPDATE CASCADE;

-- AddForeignKey
ALTER TABLE "guest_user_on_account" ADD CONSTRAINT "guest_user_on_account_guest_user_id_fkey" FOREIGN KEY ("guest_user_id") REFERENCES "guest_user"("id") ON DELETE RESTRICT ON UPDATE CASCADE;

-- AddForeignKey
ALTER TABLE "guest_user_on_account" ADD CONSTRAINT "guest_user_on_account_account_id_fkey" FOREIGN KEY ("account_id") REFERENCES "account"("id") ON DELETE RESTRICT ON UPDATE CASCADE;
