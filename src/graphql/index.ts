import { graphqlHTTP } from 'express-graphql';
import { makeExecutableSchema } from '@graphql-tools/schema';
import typeDefs from './schema';
import boost from './boost';
import boosts from './boosts';
import status from './status';

const rootValue = {
  Query: { boost, boosts, status }
};

const schema = makeExecutableSchema({ typeDefs, resolvers: rootValue });

export default graphqlHTTP({
  schema,
  rootValue,
  graphiql: true
});
