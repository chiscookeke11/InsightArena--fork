import { MigrationInterface, QueryRunner } from "typeorm";

export class CreateUserEntity1774313247489 implements MigrationInterface {
    name = 'CreateUserEntity1774313247489'

    public async up(queryRunner: QueryRunner): Promise<void> {
        await queryRunner.query(`CREATE TABLE "users" ("id" uuid NOT NULL DEFAULT uuid_generate_v4(), "stellar_address" character varying NOT NULL, "username" character varying, "avatar_url" character varying, "total_predictions" integer NOT NULL DEFAULT '0', "correct_predictions" integer NOT NULL DEFAULT '0', "total_staked_stroops" bigint NOT NULL DEFAULT '0', "total_winnings_stroops" bigint NOT NULL DEFAULT '0', "reputation_score" integer NOT NULL DEFAULT '0', "season_points" integer NOT NULL DEFAULT '0', "role" character varying NOT NULL DEFAULT 'user', "created_at" TIMESTAMP NOT NULL DEFAULT now(), "updated_at" TIMESTAMP NOT NULL DEFAULT now(), CONSTRAINT "UQ_42e18df61bbf80f10c661156e11" UNIQUE ("stellar_address"), CONSTRAINT "PK_a3ffb1c0c8416b9fc6f907b7433" PRIMARY KEY ("id"))`);
        await queryRunner.query(`CREATE INDEX "IDX_42e18df61bbf80f10c661156e1" ON "users" ("stellar_address") `);
    }

    public async down(queryRunner: QueryRunner): Promise<void> {
        await queryRunner.query(`DROP INDEX "public"."IDX_42e18df61bbf80f10c661156e1"`);
        await queryRunner.query(`DROP TABLE "users"`);
    }

}
