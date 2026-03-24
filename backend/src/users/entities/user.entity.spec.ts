import { validate } from 'class-validator';
import { User } from './user.entity';

describe('UserEntity validation', () => {
  it('should validate a correct user object', async () => {
    const user = new User();
    user.stellar_address = 'GABCDEFGHIJKLMNOPQRSTUVWXYZ1234567890';
    user.username = 'testuser';
    user.total_predictions = 0;
    user.correct_predictions = 0;
    user.total_staked_stroops = '0';
    user.total_winnings_stroops = '0';
    user.reputation_score = 0;
    user.season_points = 0;
    user.role = 'user';

    const errors = await validate(user);
    expect(errors.length).toBe(0);
  });

  it('should fail validation if stellar_address is missing', async () => {
    const user = new User();
    user.username = 'testuser';
    
    const errors = await validate(user);
    expect(errors.length).toBeGreaterThan(0);
    expect(errors[0].property).toBe('stellar_address');
  });

  it('should fail validation if role is invalid', async () => {
    const user = new User();
    user.stellar_address = 'GASDFASDF';
    user.role = 'superadmin';
    
    const errors = await validate(user);
    expect(errors.length).toBeGreaterThan(0);
    expect(errors.find(e => e.property === 'role')).toBeDefined();
  });
});
