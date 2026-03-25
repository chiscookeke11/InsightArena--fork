import { Injectable } from '@nestjs/common';
import { InjectRepository } from '@nestjs/typeorm';
import { Repository } from 'typeorm';
import { Market } from './entities/market.entity';
import { UsersService } from '../users/users.service';

@Injectable()
export class MarketsService {
  constructor(
    @InjectRepository(Market)
    private readonly marketsRepository: Repository<Market>,
    private readonly usersService: UsersService,
  ) {}

  async findAll(): Promise<Market[]> {
    return this.marketsRepository.find({
      relations: ['creator'],
    });
  }

  async findById(id: string): Promise<Market | null> {
    return this.marketsRepository.findOne({
      where: { id },
      relations: ['creator'],
    });
  }
}
